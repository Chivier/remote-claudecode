use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth::jwt;
use crate::auth::middleware::{AppState, AuthUser};

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    token: String,
    user: UserResponse,
}

#[derive(Serialize)]
pub struct UserResponse {
    id: i64,
    username: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let user = state
        .db
        .get_user_by_username(&req.username)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid username or password"})),
            )
        })?;

    // Verify password
    let valid = bcrypt::verify(&req.password, &user.password_hash).unwrap_or(false);
    if !valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid username or password"})),
        ));
    }

    // Update last login
    state.db.update_last_login(user.id);

    // Generate token
    let token = jwt::generate_token(user.id, &user.username, &state.jwt_secret).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    Ok(Json(json!({
        "token": token,
        "user": {
            "id": user.id,
            "username": user.username
        }
    })))
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check if users already exist (single user system)
    let has_users = state.db.has_users().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    if has_users {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Registration is closed. A user already exists."})),
        ));
    }

    // Hash password
    let password_hash = bcrypt::hash(&req.password, 10).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    // Create user
    let user = state.db.create_user(&req.username, &password_hash).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    // Ensure local server exists for this user
    state.db.ensure_local_server(user.id).ok();

    // Generate token
    let token = jwt::generate_token(user.id, &user.username, &state.jwt_secret).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    Ok(Json(json!({
        "token": token,
        "user": {
            "id": user.id,
            "username": user.username
        }
    })))
}

pub async fn check_auth(auth: AuthUser) -> Json<Value> {
    Json(json!({
        "authenticated": true,
        "user": {
            "id": auth.user_id,
            "username": auth.username
        }
    }))
}

pub async fn has_users(State(state): State<AppState>) -> Json<Value> {
    let has = state.db.has_users().unwrap_or(false);
    Json(json!({"hasUsers": has}))
}
