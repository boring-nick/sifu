use crate::config::AuthConfig;
use axum::{
    http::{
        header::{AUTHORIZATION, WWW_AUTHENTICATE},
        HeaderMap, Request, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::string::FromUtf8Error;
use thiserror::Error;

pub async fn auth<B>(req: Request<B>, next: Next<B>) -> Result<Response, AuthError> {
    match req
        .extensions()
        .get::<AuthConfig>()
        .expect("AuthConfig not attached")
    {
        AuthConfig::Enabled {
            username: config_username,
            password: config_password,
        } => {
            let auth_header = req
                .headers()
                .get(AUTHORIZATION)
                .and_then(|header| header.to_str().ok())
                .and_then(|s| s.strip_prefix("Basic "))
                .ok_or(AuthError::MissingAuth)?;

            let auth_data = base64::decode(auth_header)?;
            let auth_string = String::from_utf8(auth_data)?;

            let (username, password) = auth_string
                .split_once(':')
                .ok_or(AuthError::InvalidCredentials)?;

            if username.trim() == config_username && password.trim() == config_password {
                Ok(next.run(req).await)
            } else {
                Err(AuthError::InvalidCredentials)
            }
        }
        AuthConfig::Disabled => Ok(next.run(req).await),
    }
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid username or password")]
    InvalidCredentials,
    #[error("Invalid base64 provided")]
    InvalidBase64(#[from] base64::DecodeError),
    #[error("Invalid UTF-8")]
    Utf8(#[from] FromUtf8Error),
    #[error("Auth not provided")]
    MissingAuth,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert(WWW_AUTHENTICATE, "Basic".parse().unwrap());

        (StatusCode::UNAUTHORIZED, headers, self.to_string()).into_response()
    }
}
