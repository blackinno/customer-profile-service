pub mod controller;
pub use controller::routes;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        controller::upload_profile_image,
        controller::get_profile_image,
        controller::delete_profile_image,
    ),
    components(schemas(
        application::profile_images::dtos::ProfileImageResponse,
    )),
    tags((name = "ProfileImages", description = "Customer profile image upload and management")),
)]
pub struct DomainDoc;
