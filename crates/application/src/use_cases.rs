use std::sync::Arc;

use crate::customers::use_cases::CustomerUseCases;
use crate::identities::use_cases::IdentityUseCases;
use crate::profile_changes::use_cases::ProfileChangeUseCases;
use crate::profile_images::use_cases::ProfileImageUseCases;
use crate::segments::use_cases::SegmentUseCases;
use crate::the1::use_cases::The1UseCases;

#[derive(Clone)]
pub struct UseCases {
    pub customers: Arc<CustomerUseCases>,
    pub identities: Arc<IdentityUseCases>,
    pub profile_changes: Arc<ProfileChangeUseCases>,
    pub profile_images: Arc<ProfileImageUseCases>,
    pub segments: Arc<SegmentUseCases>,
    pub the1: Arc<The1UseCases>,
}
