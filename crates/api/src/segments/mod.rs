pub mod controller;
pub use controller::routes;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        controller::get_segment,
    ),
    components(schemas(
        application::segments::dtos::SegmentResponse,
        crate::segments::controller::SegmentQuery,
    )),
    tags((name = "Segments", description = "Customer tier/segment lookup via The1 card")),
)]
pub struct DomainDoc;
