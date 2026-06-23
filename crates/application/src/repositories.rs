use std::sync::Arc;

use domain::repositories::customer_repository::CustomerRepository;
use domain::repositories::identity_repository::IdentityRepository;
use domain::repositories::profile_change_repository::ProfileChangeRepository;
use domain::repositories::the1_user_repository::The1UserRepository;

#[derive(Clone)]
pub struct Repositories {
    pub customers: Arc<dyn CustomerRepository>,
    pub identities: Arc<dyn IdentityRepository>,
    pub profile_changes: Arc<dyn ProfileChangeRepository>,
    pub the1_users: Arc<dyn The1UserRepository>,
}
