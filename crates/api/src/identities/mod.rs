pub mod controller;
pub use controller::routes;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        controller::get_my_identities,
        controller::create_identity,
        controller::delete_identity,
        controller::invoke_token,
        controller::get_identities_internal,
    ),
    components(schemas(
        application::identities::dtos::CreateIdentityRequest,
        application::identities::dtos::IdentityResponse,
        application::identities::dtos::InvokeTokenResponse,
    )),
    tags((name = "Identities", description = "Customer identity/provider management")),
)]
pub struct DomainDoc;
