pub mod controller;
pub use controller::routes;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        controller::get_the1_account,
    ),
    components(schemas(
        application::the1::dtos::The1AccountResponse,
        application::the1::dtos::TierResponse,
        crate::the1::controller::The1AccountQuery,
    )),
    tags((name = "The1", description = "The1 loyalty account integration")),
)]
pub struct DomainDoc;
