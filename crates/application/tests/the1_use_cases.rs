use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use domain::{
    entities::the1_user::{The1User, UpsertThe1User},
    errors::RepositoryError,
    repositories::the1_user_repository::The1UserRepository,
};
use uuid::Uuid;

use application::errors::ApplicationError;
use application::the1::use_cases::The1UseCases;

fn make_the1_user(id: Uuid, user_uuid: Uuid) -> The1User {
    The1User {
        id,
        user_uuid,
        member_id: "member-1".to_string(),
        account_id: "account-1".to_string(),
        profile_id: "profile-1".to_string(),
        card_number: Some("1234567890".to_string()),
        tiers: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

struct MockThe1Repo {
    by_user: Option<The1User>,
    by_card: Option<The1User>,
    by_member: Option<The1User>,
}

#[async_trait]
impl The1UserRepository for MockThe1Repo {
    async fn find_by_user(&self, _: Uuid) -> Result<Option<The1User>, RepositoryError> {
        Ok(self.by_user.clone())
    }
    async fn find_by_card_number(&self, _: &str) -> Result<Option<The1User>, RepositoryError> {
        Ok(self.by_card.clone())
    }
    async fn find_by_member_id(&self, _: &str) -> Result<Option<The1User>, RepositoryError> {
        Ok(self.by_member.clone())
    }
    async fn upsert(&self, _: Uuid, _: UpsertThe1User) -> Result<The1User, RepositoryError> {
        Ok(self.by_user.clone().unwrap())
    }
}

fn use_cases(repo: MockThe1Repo) -> The1UseCases {
    The1UseCases::new(Arc::new(repo))
}

#[tokio::test]
async fn get_the1_account_found() {
    let user_uuid = Uuid::new_v4();
    let repo = MockThe1Repo {
        by_user: Some(make_the1_user(Uuid::new_v4(), user_uuid)),
        by_card: None,
        by_member: None,
    };
    let res = use_cases(repo).get_the1_account(user_uuid).await.unwrap();
    assert_eq!(res.member_id, "member-1");
}

#[tokio::test]
async fn get_the1_account_not_found() {
    let repo = MockThe1Repo { by_user: None, by_card: None, by_member: None };
    let err = use_cases(repo).get_the1_account(Uuid::new_v4()).await.unwrap_err();
    assert!(matches!(err, ApplicationError::NotFound(_)));
}

#[tokio::test]
async fn get_by_card_number_returns_some() {
    let user_uuid = Uuid::new_v4();
    let repo = MockThe1Repo {
        by_user: None,
        by_card: Some(make_the1_user(Uuid::new_v4(), user_uuid)),
        by_member: None,
    };
    let res = use_cases(repo).get_by_card_number("1234567890").await.unwrap();
    assert!(res.is_some());
}

#[tokio::test]
async fn get_by_card_number_returns_none() {
    let repo = MockThe1Repo { by_user: None, by_card: None, by_member: None };
    let res = use_cases(repo).get_by_card_number("unknown").await.unwrap();
    assert!(res.is_none());
}

#[tokio::test]
async fn get_by_member_id_returns_some() {
    let user_uuid = Uuid::new_v4();
    let repo = MockThe1Repo {
        by_user: None,
        by_card: None,
        by_member: Some(make_the1_user(Uuid::new_v4(), user_uuid)),
    };
    let res = use_cases(repo).get_by_member_id("member-1").await.unwrap();
    assert!(res.is_some());
}

#[tokio::test]
async fn get_by_member_id_returns_none() {
    let repo = MockThe1Repo { by_user: None, by_card: None, by_member: None };
    let res = use_cases(repo).get_by_member_id("unknown").await.unwrap();
    assert!(res.is_none());
}
