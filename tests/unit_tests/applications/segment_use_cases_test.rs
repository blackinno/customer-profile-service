#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;

    use application::errors::ApplicationError;
    use application::segments::use_cases::{SegmentUseCases, The1Client, The1PartnerMemberData};
    use domain::entities::the1_user::{The1User, Tier, UpsertTier, UpsertThe1User};
    use domain::errors::RepositoryError;
    use domain::repositories::the1_user_repository::The1UserRepository;

    // ─────────────────────────────── Mock: The1UserRepository ────────────────

    /// Configurable mock for The1UserRepository. Only `upsert` is exercised by
    /// `SegmentUseCases`; other methods panic if unexpectedly called.
    struct MockThe1UserRepo {
        /// `Some(user)` → return that user on upsert success.
        /// `None` → return a Backend error.
        upsert_user: Option<The1User>,
    }

    #[async_trait]
    impl The1UserRepository for MockThe1UserRepo {
        async fn find_by_user(&self, _: Uuid) -> Result<Option<The1User>, RepositoryError> {
            panic!("find_by_user not expected in segment tests")
        }

        async fn find_by_card_number(
            &self,
            _: &str,
        ) -> Result<Option<The1User>, RepositoryError> {
            panic!("find_by_card_number not expected in segment tests")
        }

        async fn find_by_member_id(
            &self,
            _: &str,
        ) -> Result<Option<The1User>, RepositoryError> {
            panic!("find_by_member_id not expected in segment tests")
        }

        async fn upsert(
            &self,
            user_uuid: Uuid,
            _: UpsertThe1User,
        ) -> Result<The1User, RepositoryError> {
            match &self.upsert_user {
                Some(template) => Ok(The1User {
                    id: template.id,
                    user_uuid,
                    member_id: template.member_id.clone(),
                    account_id: template.account_id.clone(),
                    profile_id: template.profile_id.clone(),
                    card_number: template.card_number.clone(),
                    tiers: template.tiers.clone(),
                    created_at: template.created_at,
                    updated_at: template.updated_at,
                }),
                None => Err(RepositoryError::Backend("mock db error".to_string())),
            }
        }
    }

    // ────────────────────────────── Mock: The1Client ──────────────────────────

    struct MockThe1Client {
        /// `Some(data)` → return that data on success.
        /// `None` → return an error string.
        partner_data: Option<The1PartnerMemberData>,
    }

    #[async_trait]
    impl The1Client for MockThe1Client {
        async fn get_partner_member(
            &self,
            card_number: &str,
        ) -> Result<The1PartnerMemberData, String> {
            match &self.partner_data {
                Some(d) => Ok(The1PartnerMemberData {
                    user_uuid: d.user_uuid,
                    member_id: d.member_id.clone(),
                    account_id: d.account_id.clone(),
                    profile_id: d.profile_id.clone(),
                    card_number: Some(card_number.to_string()),
                    tiers: d.tiers.clone(),
                }),
                None => Err("external service unavailable".to_string()),
            }
        }
    }

    // ──────────────────────────────── Helpers ─────────────────────────────────

    fn gold_upsert_tier() -> UpsertTier {
        UpsertTier {
            code: "GOLD".to_string(),
            name: Some("Gold Member".to_string()),
            expired_date: None,
        }
    }

    fn gold_tier() -> Tier {
        Tier {
            id: Uuid::new_v4(),
            code: "GOLD".to_string(),
            name: Some("Gold Member".to_string()),
            expired_date: None,
        }
    }

    fn make_the1_user(user_uuid: Uuid, tiers: Vec<Tier>) -> The1User {
        The1User {
            id: Uuid::new_v4(),
            user_uuid,
            member_id: "MEM001".to_string(),
            account_id: "ACC001".to_string(),
            profile_id: "PRO001".to_string(),
            card_number: Some("1234567890123456".to_string()),
            tiers,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_partner_data(user_uuid: Uuid, tiers: Vec<UpsertTier>) -> The1PartnerMemberData {
        The1PartnerMemberData {
            user_uuid,
            member_id: "MEM001".to_string(),
            account_id: "ACC001".to_string(),
            profile_id: "PRO001".to_string(),
            card_number: Some("1234567890123456".to_string()),
            tiers,
        }
    }

    // ──────────────────────────────── Tests ───────────────────────────────────

    #[tokio::test]
    async fn test_get_segment_happy_path_returns_first_tier() {
        let user_uuid = Uuid::new_v4();

        let repo = MockThe1UserRepo {
            upsert_user: Some(make_the1_user(user_uuid, vec![gold_tier()])),
        };
        let client = MockThe1Client {
            partner_data: Some(make_partner_data(user_uuid, vec![gold_upsert_tier()])),
        };

        let uc = SegmentUseCases::new(Arc::new(repo), Arc::new(client));
        let result = uc.get_segment("1234567890123456".to_string()).await;

        assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
        let seg = result.unwrap();
        assert_eq!(seg.segment_slug, "GOLD");
        assert_eq!(seg.user_uuid, user_uuid);
        assert!(seg.expired_time.is_none());
    }

    #[tokio::test]
    async fn test_get_segment_multiple_tiers_returns_first() {
        let user_uuid = Uuid::new_v4();
        let tiers = vec![
            Tier {
                id: Uuid::new_v4(),
                code: "FIRST".to_string(),
                name: None,
                expired_date: None,
            },
            Tier {
                id: Uuid::new_v4(),
                code: "SECOND".to_string(),
                name: None,
                expired_date: None,
            },
        ];

        let repo = MockThe1UserRepo {
            upsert_user: Some(make_the1_user(user_uuid, tiers)),
        };
        let client = MockThe1Client {
            partner_data: Some(make_partner_data(user_uuid, vec![
                UpsertTier { code: "FIRST".to_string(), name: None, expired_date: None },
                UpsertTier { code: "SECOND".to_string(), name: None, expired_date: None },
            ])),
        };

        let uc = SegmentUseCases::new(Arc::new(repo), Arc::new(client));
        let result = uc.get_segment("CARD123".to_string()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().segment_slug, "FIRST");
    }

    #[tokio::test]
    async fn test_get_segment_no_tiers_returns_not_found() {
        let user_uuid = Uuid::new_v4();

        let repo = MockThe1UserRepo {
            upsert_user: Some(make_the1_user(user_uuid, vec![])), // empty tiers
        };
        let client = MockThe1Client {
            partner_data: Some(make_partner_data(user_uuid, vec![])),
        };

        let uc = SegmentUseCases::new(Arc::new(repo), Arc::new(client));
        let result = uc.get_segment("1234567890123456".to_string()).await;

        match result {
            Err(ApplicationError::NotFound(msg)) => {
                assert!(msg.contains("no segment found"), "unexpected msg: {msg}")
            }
            other => panic!("expected NotFound, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_get_segment_external_client_error_returns_external() {
        let user_uuid = Uuid::new_v4();
        let repo = MockThe1UserRepo {
            upsert_user: Some(make_the1_user(user_uuid, vec![])),
        };
        let client = MockThe1Client { partner_data: None }; // simulate HTTP failure

        let uc = SegmentUseCases::new(Arc::new(repo), Arc::new(client));
        let result = uc.get_segment("bad_card".to_string()).await;

        match result {
            Err(ApplicationError::External(_)) => {}
            other => panic!("expected External, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_get_segment_repository_upsert_error_propagates() {
        let user_uuid = Uuid::new_v4();
        let repo = MockThe1UserRepo { upsert_user: None }; // db failure
        let client = MockThe1Client {
            partner_data: Some(make_partner_data(user_uuid, vec![gold_upsert_tier()])),
        };

        let uc = SegmentUseCases::new(Arc::new(repo), Arc::new(client));
        let result = uc.get_segment("1234567890123456".to_string()).await;

        match result {
            Err(ApplicationError::Repository(_)) => {}
            other => panic!("expected Repository, got: {:?}", other),
        }
    }
}
