use std::sync::Arc;

use application::errors::ApplicationError;
use application::segments::use_cases::{SegmentUseCases, The1Client, The1PartnerMemberData};
use async_trait::async_trait;
use chrono::Utc;
use domain::{
    entities::the1_user::{The1User, Tier, UpsertThe1User, UpsertTier},
    errors::RepositoryError,
    repositories::the1_user_repository::The1UserRepository,
};
use uuid::Uuid;

fn make_the1_user(user_uuid: Uuid, tiers: Vec<Tier>) -> The1User {
    The1User {
        id: Uuid::new_v4(),
        user_uuid,
        member_id: "member-1".to_string(),
        account_id: "account-1".to_string(),
        profile_id: "profile-1".to_string(),
        card_number: Some("1234567890".to_string()),
        tiers,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn make_tier(code: &str) -> Tier {
    Tier { id: Uuid::new_v4(), code: code.to_string(), name: None, expired_date: None }
}

struct MockThe1Repo { upsert_result: The1User }

#[async_trait]
impl The1UserRepository for MockThe1Repo {
    async fn find_by_user(&self, _: Uuid) -> Result<Option<The1User>, RepositoryError> { Ok(None) }
    async fn find_by_card_number(&self, _: &str) -> Result<Option<The1User>, RepositoryError> { Ok(None) }
    async fn find_by_member_id(&self, _: &str) -> Result<Option<The1User>, RepositoryError> { Ok(None) }
    async fn upsert(&self, _: Uuid, _: UpsertThe1User) -> Result<The1User, RepositoryError> {
        Ok(self.upsert_result.clone())
    }
}

struct MockThe1Client { result: Result<The1PartnerMemberData, String> }

#[async_trait]
impl The1Client for MockThe1Client {
    async fn get_partner_member(&self, _: &str) -> Result<The1PartnerMemberData, String> {
        self.result.as_ref().map(|d| The1PartnerMemberData {
            user_uuid: d.user_uuid,
            member_id: d.member_id.clone(),
            account_id: d.account_id.clone(),
            profile_id: d.profile_id.clone(),
            card_number: d.card_number.clone(),
            tiers: d.tiers.clone(),
        }).map_err(|e| e.clone())
    }
}

fn make_partner_data(user_uuid: Uuid, tiers: Vec<UpsertTier>) -> The1PartnerMemberData {
    The1PartnerMemberData {
        user_uuid,
        member_id: "member-1".to_string(),
        account_id: "account-1".to_string(),
        profile_id: "profile-1".to_string(),
        card_number: Some("1234567890".to_string()),
        tiers,
    }
}

#[tokio::test]
async fn get_segment_returns_first_tier() {
    let user_uuid = Uuid::new_v4();
    let tiers = vec![make_tier("GOLD"), make_tier("SILVER")];
    let the1_user = make_the1_user(user_uuid, tiers);
    let data = make_partner_data(user_uuid, vec![
        UpsertTier { code: "GOLD".to_string(), name: None, expired_date: None },
    ]);
    let uc = SegmentUseCases::new(
        Arc::new(MockThe1Repo { upsert_result: the1_user }),
        Arc::new(MockThe1Client { result: Ok(data) }),
    );
    let res = uc.get_segment("1234567890".to_string()).await.unwrap();
    assert_eq!(res.segment_slug, "GOLD");
}

#[tokio::test]
async fn get_segment_no_tiers_returns_not_found() {
    let user_uuid = Uuid::new_v4();
    let the1_user = make_the1_user(user_uuid, vec![]); // no tiers
    let data = make_partner_data(user_uuid, vec![]);
    let uc = SegmentUseCases::new(
        Arc::new(MockThe1Repo { upsert_result: the1_user }),
        Arc::new(MockThe1Client { result: Ok(data) }),
    );
    let err = uc.get_segment("1234567890".to_string()).await.unwrap_err();
    assert!(matches!(err, ApplicationError::NotFound(_)));
}

#[tokio::test]
async fn get_segment_external_error_propagates() {
    let user_uuid = Uuid::new_v4();
    let the1_user = make_the1_user(user_uuid, vec![]);
    let uc = SegmentUseCases::new(
        Arc::new(MockThe1Repo { upsert_result: the1_user }),
        Arc::new(MockThe1Client { result: Err("the1 api down".to_string()) }),
    );
    let err = uc.get_segment("bad-card".to_string()).await.unwrap_err();
    assert!(matches!(err, ApplicationError::External(_)));
}
