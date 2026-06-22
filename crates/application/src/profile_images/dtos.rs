use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProfileImageResponse {
    pub url: String,
}
