use axum::{
    extract::multipart::MultipartError,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;
use tracing::error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error")]
    IoError(#[from] std::io::Error),
    #[error("Multipart error: {0}")]
    Multipart(#[from] MultipartError),
    #[error("File not found")]
    FileNotFound,
    #[error("Bad request: {0}")]
    BadRequest(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let code = match &self {
            Error::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Multipart(_) => StatusCode::BAD_REQUEST,
            Error::FileNotFound => StatusCode::NOT_FOUND,
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
        };

        (code, self.to_string()).into_response()
    }
}
