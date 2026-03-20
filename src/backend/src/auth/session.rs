use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

pub const USER_KEY: &str = "user";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub sub: String,
    pub name: String,
    pub email: String,
    pub roles: Vec<String>,
}

impl User {
    pub fn anonymous() -> Self {
        Self {
            sub: "anonymous".to_string(),
            name: "Anonymous".to_string(),
            email: String::new(),
            roles: vec!["*".to_string()],
        }
    }

    pub fn guest() -> Self {
        Self {
            sub: "guest".to_string(),
            name: "Guest".to_string(),
            email: String::new(),
            roles: Vec::new(),
        }
    }
}

impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<User>()
            .cloned()
            .ok_or_else(|| StatusCode::UNAUTHORIZED.into_response())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("invalid state parameter")]
    InvalidState,
    #[error("token exchange failed: {0}")]
    TokenExchangeFailed(String),
    #[error("provider error: {0}")]
    ProviderError(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = match &self {
            AuthError::InvalidState => StatusCode::BAD_REQUEST,
            AuthError::TokenExchangeFailed(_) | AuthError::ProviderError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        tracing::error!("auth error: {self}");
        status.into_response()
    }
}
