pub mod controller;
pub use controller::routes;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        controller::create_profile_change,
        controller::update_profile_change,
        controller::verify_profile_change,
        controller::confirm_profile_change,
    ),
    components(schemas(
        application::profile_changes::dtos::CreateProfileChangeRequest,
        application::profile_changes::dtos::UpdateProfileChangeRequest,
        application::profile_changes::dtos::VerifyProfileChangeRequest,
        application::profile_changes::dtos::ProfileChangeResponse,
    )),
    tags((name = "ProfileChanges", description = "Customer profile change requests (OTP-based)")),
)]
pub struct DomainDoc;
