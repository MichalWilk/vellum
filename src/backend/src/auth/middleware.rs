use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use tower_sessions::Session;

use crate::auth::User;
use crate::auth::session::USER_KEY;
use crate::config::AuthMode;

pub async fn require_auth(
    session: Session,
    auth_mode: AuthMode,
    mut req: Request,
    next: Next,
) -> Response {
    match auth_mode {
        AuthMode::None => {
            req.extensions_mut().insert(User::anonymous());
            next.run(req).await
        }
        AuthMode::Oidc => match session.get::<User>(USER_KEY).await {
            Ok(Some(user)) => {
                req.extensions_mut().insert(user);
                next.run(req).await
            }
            _ => axum::http::StatusCode::UNAUTHORIZED.into_response(),
        },
    }
}

pub async fn optional_auth(
    session: Session,
    auth_mode: AuthMode,
    mut req: Request,
    next: Next,
) -> Response {
    match auth_mode {
        AuthMode::None => {
            req.extensions_mut().insert(User::anonymous());
        }
        AuthMode::Oidc => {
            let user = session
                .get::<User>(USER_KEY)
                .await
                .ok()
                .flatten()
                .unwrap_or_else(User::guest);
            req.extensions_mut().insert(user);
        }
    }
    next.run(req).await
}

