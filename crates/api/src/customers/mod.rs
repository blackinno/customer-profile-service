pub mod controller;
pub use controller::routes;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        controller::create_customer,
        controller::search_customers,
        controller::get_me,
        controller::update_me,
        controller::get_customer_by_id,
        controller::delete_customer,
    ),
    components(schemas(
        application::customers::dtos::CreateCustomerRequest,
        application::customers::dtos::UpdateCustomerRequest,
        application::customers::dtos::CustomerResponse,
        application::customers::dtos::SearchCustomerQuery,
    )),
    tags((name = "Customers", description = "Customer profile management")),
)]
pub struct DomainDoc;
