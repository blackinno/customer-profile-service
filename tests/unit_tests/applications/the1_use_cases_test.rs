#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;

    use application::errors::ApplicationError;
    use application::the1::use_cases::The1UseCases;
    use domain::entities::the1_user::{The1User, Tier, UpsertThe1User};
    use domain::errors::RepositoryError;
    use domain::repositories::the1_user_repository::The1UserRepository;

    // ─────────────────────────────── Mock ─────────────────────────────────────

    /// Configurable mock for all three read paths on The1UserRepository.
    /// Set each `Option` to control what the mock returns:
    ///   `Some(user)` → found,  `None` → not found,  combined with `*_err` for errors.
    struct MockThe1UserRepo {
        by_user: Option<The1User>,
        by_user_err: bool,
        by_card: Option<The1User>,
        by_card_err: bool,
        by_member: Option<The1User>,
        by_member_err: bool,
    }

    impl Default for MockThe1UserRepo {
        fn default() -> Self {
            Self {
                by_user: None,
                by_user_err: false,
                by_card: None,
                by_card_err: false,
                by_member: None,
                by_member_err: false,
            }
        }
    }

    #[async_trait]
    impl The1UserRepository for MockThe1UserRepo {
        async fn find_by_user(
            &self,
            _: Uuid,
        ) -> Result<Option<The1User>, RepositoryError> {
            if self.by_user_err {
                return Err(RepositoryError::Backend("mock db error".to_string()));
            }
            Ok(self.by_user.clone())
        }

        async fn find_by_card_number(
            &self,
            _: &str,
        ) -> Result<Option<The1User>, RepositoryError> {
            if self.by_card_err {
                return Err(RepositoryError::Backend("mock db error".to_string()));
            }
            Ok(self.by_card.clone())
        }

        async fn find_by_member_id(
            &self,
            _: &str,
        ) -> Result<Option<The1User>, RepositoryError> {
            if self.by_member_err {
                return Err(RepositoryError::Backend("mock db error".to_string()));
            }
            Ok(self.by_member.clone())
        }

        async fn upsert(
            &self,
            _: Uuid,
            _: UpsertThe1User,
        ) -> Result<The1User, RepositoryError> {
            panic!("upsert not expected in the1 use-case tests")
        }
    }

    // ──────────────────────────────── Helpers ─────────────────────────────────

    fn make_the1_user_with_tiers(user_uuid: Uuid) -> The1User {
        The1User {
            id: Uuid::new_v4(),
            user_uuid,
            member_id: "MEM001".to_string(),
            account_id: "ACC001".to_string(),
            profile_id: "PRO001".to_string(),
            card_number: Some("4567890123456789".to_string()),
            tiers: vec![Tier {
                id: Uuid::new_v4(),
                code: "GOLD".to_string(),
                name: Some("Gold Member".to_string()),
                expired_date: None,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_the1_user_no_tiers(user_uuid: Uuid) -> The1User {
        The1User {
            id: Uuid::new_v4(),
            user_uuid,
            member_id: "MEM002".to_string(),
            account_id: "ACC002".to_string(),
            profile_id: "PRO002".to_string(),
            card_number: None,
            tiers: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ─────────────────── get_the1_account tests ───────────────────────────────

    #[tokio::test]
    async fn test_get_the1_account_found_returns_response() {
        let user_uuid = Uuid::new_v4();
        let repo = MockThe1UserRepo {
            by_user: Some(make_the1_user_with_tiers(user_uuid)),
            ..Default::default()
        };

        let uc = The1UseCases::new(Arc::new(repo));
        let result = uc.get_the1_account(user_uuid).await;

        assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
        let acc = result.unwrap();
        assert_eq!(acc.user_uuid, user_uuid);
        assert_eq!(acc.member_id, "MEM001");
        assert_eq!(acc.tiers.len(), 1);
        assert_eq!(acc.tiers[0].code, "GOLD");
    }

    #[tokio::test]
    async fn test_get_the1_account_not_found_returns_error() {
        let repo = MockThe1UserRepo {
            by_user: None, // not found
            ..Default::default()
        };

        let uc = The1UseCases::new(Arc::new(repo));
        let result = uc.get_the1_account(Uuid::new_v4()).await;

        match result {
            Err(ApplicationError::NotFound(msg)) => {
                assert!(
                    msg.contains("the1 account not found"),
                    "unexpected msg: {msg}"
                )
            }
            other => panic!("expected NotFound, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_get_the1_account_repository_error_propagates() {
        let repo = MockThe1UserRepo {
            by_user_err: true,
            ..Default::default()
        };

        let uc = The1UseCases::new(Arc::new(repo));
        let result = uc.get_the1_account(Uuid::new_v4()).await;

        match result {
            Err(ApplicationError::Repository(_)) => {}
            other => panic!("expected Repository error, got: {:?}", other),
        }
    }

    // ─────────────────── get_by_card_number tests ─────────────────────────────

    #[tokio::test]
    async fn test_get_by_card_number_found_returns_some() {
        let user_uuid = Uuid::new_v4();
        let repo = MockThe1UserRepo {
            by_card: Some(make_the1_user_with_tiers(user_uuid)),
            ..Default::default()
        };

        let uc = The1UseCases::new(Arc::new(repo));
        let result = uc.get_by_card_number("4567890123456789").await;

        assert!(result.is_ok());
        let maybe = result.unwrap();
        assert!(maybe.is_some());
        let acc = maybe.unwrap();
        assert_eq!(acc.user_uuid, user_uuid);
    }

    #[tokio::test]
    async fn test_get_by_card_number_not_found_returns_none() {
        let repo = MockThe1UserRepo {
            by_card: None,
            ..Default::default()
        };

        let uc = The1UseCases::new(Arc::new(repo));
        let result = uc.get_by_card_number("unknown_card").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_by_card_number_repository_error_propagates() {
        let repo = MockThe1UserRepo {
            by_card_err: true,
            ..Default::default()
        };

        let uc = The1UseCases::new(Arc::new(repo));
        let result = uc.get_by_card_number("any_card").await;

        match result {
            Err(ApplicationError::Repository(_)) => {}
            other => panic!("expected Repository error, got: {:?}", other),
        }
    }

    // ─────────────────── get_by_member_id tests ───────────────────────────────

    #[tokio::test]
    async fn test_get_by_member_id_found_returns_some() {
        let user_uuid = Uuid::new_v4();
        let repo = MockThe1UserRepo {
            by_member: Some(make_the1_user_no_tiers(user_uuid)),
            ..Default::default()
        };

        let uc = The1UseCases::new(Arc::new(repo));
        let result = uc.get_by_member_id("MEM002").await;

        assert!(result.is_ok());
        let maybe = result.unwrap();
        assert!(maybe.is_some());
        let acc = maybe.unwrap();
        assert_eq!(acc.member_id, "MEM002");
        assert!(acc.tiers.is_empty());
    }

    #[tokio::test]
    async fn test_get_by_member_id_not_found_returns_none() {
        let repo = MockThe1UserRepo {
            by_member: None,
            ..Default::default()
        };

        let uc = The1UseCases::new(Arc::new(repo));
        let result = uc.get_by_member_id("unknown_member").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_by_member_id_repository_error_propagates() {
        let repo = MockThe1UserRepo {
            by_member_err: true,
            ..Default::default()
        };

        let uc = The1UseCases::new(Arc::new(repo));
        let result = uc.get_by_member_id("any_member").await;

        match result {
            Err(ApplicationError::Repository(_)) => {}
            other => panic!("expected Repository error, got: {:?}", other),
        }
    }

    // ──────────────── DTO conversion tests ────────────────────────────────────

    #[tokio::test]
    async fn test_get_the1_account_dto_timestamps_are_rfc3339() {
        let user_uuid = Uuid::new_v4();
        let repo = MockThe1UserRepo {
            by_user: Some(make_the1_user_with_tiers(user_uuid)),
            ..Default::default()
        };

        let uc = The1UseCases::new(Arc::new(repo));
        let acc = uc.get_the1_account(user_uuid).await.unwrap();

        // RFC-3339 timestamps should parse back without error
        chrono::DateTime::parse_from_rfc3339(&acc.created_at)
            .expect("created_at should be valid RFC-3339");
        chrono::DateTime::parse_from_rfc3339(&acc.updated_at)
            .expect("updated_at should be valid RFC-3339");
    }
}
