use serde::Serialize;

#[derive(Serialize)]
pub struct UploadResponse {
    pub full_url: String,
    pub file_name: String,
}
