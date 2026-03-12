use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use super::jwt;
use crate::db::Database;

/// Authenticated user extracted from JWT token
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i64,
    pub username: String,
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub jwt_secret: String,
    pub config: Arc<crate::config::Config>,
}

#[derive(Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        // Platform mode: use first user
        if app_state.config.is_platform {
            return match app_state.db.get_first_user() {
                Ok(Some(user)) => Ok(AuthUser {
                    user_id: user.id,
                    username: user.username,
                }),
                _ => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Platform mode: No user found"})),
                )
                    .into_response()),
            };
        }

        // Extract token from Authorization header or query param
        let token = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|s| s.to_string())
            .or_else(|| {
                // Check query param for SSE endpoints
                let query = parts.uri.query().unwrap_or("");
                let parsed: Result<TokenQuery, _> =
                    serde_urlencoded::from_str(query);
                parsed.ok().and_then(|q| q.token)
            });

        let token = match token {
            Some(t) => t,
            None => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "Access denied. No token provided."})),
                )
                    .into_response());
            }
        };

        // Verify JWT
        let claims = match jwt::verify_token(&token, &app_state.jwt_secret) {
            Ok(c) => c,
            Err(_) => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({"error": "Invalid token"})),
                )
                    .into_response());
            }
        };

        // Verify user still exists
        match app_state.db.get_user_by_id(claims.user_id) {
            Ok(Some(_)) => Ok(AuthUser {
                user_id: claims.user_id,
                username: claims.username,
            }),
            _ => Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid token. User not found."})),
            )
                .into_response()),
        }
    }
}

/// Helper trait for extracting AppState from nested state types
pub trait FromRef<T> {
    fn from_ref(input: &T) -> Self;
}

impl FromRef<AppState> for AppState {
    fn from_ref(input: &AppState) -> Self {
        input.clone()
    }
}

/// Authenticate a WebSocket connection from a token string
pub fn authenticate_websocket(token: Option<&str>, db: &Database, jwt_secret: &str, is_platform: bool) -> Option<AuthUser> {
    if is_platform {
        return db.get_first_user().ok().flatten().map(|u| AuthUser {
            user_id: u.id,
            username: u.username,
        });
    }

    let token = token?;
    let claims = jwt::verify_token(token, jwt_secret).ok()?;
    let user = db.get_user_by_id(claims.user_id).ok()??;
    Some(AuthUser {
        user_id: user.id,
        username: user.username,
    })
}
