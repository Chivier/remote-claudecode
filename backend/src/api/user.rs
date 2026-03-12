use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::middleware::{AppState, AuthUser};

pub async fn get_git_config(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (git_name, git_email) = state.db.get_git_config(auth.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;
    Ok(Json(json!({
        "gitName": git_name,
        "gitEmail": git_email
    })))
}

#[derive(Deserialize)]
pub struct UpdateGitConfigRequest {
    #[serde(rename = "gitName")]
    git_name: String,
    #[serde(rename = "gitEmail")]
    git_email: String,
}

pub async fn update_git_config(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateGitConfigRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    state
        .db
        .update_git_config(auth.user_id, &req.git_name, &req.git_email)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;
    Ok(Json(json!({ "success": true })))
}

pub async fn get_onboarding_status(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let completed = state
        .db
        .has_completed_onboarding(auth.user_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;
    Ok(Json(json!({ "hasCompletedOnboarding": completed })))
}

pub async fn complete_onboarding(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    state.db.complete_onboarding(auth.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;
    Ok(Json(json!({ "success": true })))
}
