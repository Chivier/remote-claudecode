use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::middleware::{AppState, AuthUser};

// --- API Keys ---

pub async fn get_api_keys(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let keys = state.db.get_api_keys(auth.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;
    Ok(Json(json!({ "apiKeys": keys })))
}

#[derive(Deserialize)]
pub struct CreateApiKeyRequest {
    #[serde(rename = "keyName")]
    key_name: String,
}

pub async fn create_api_key(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let key = state.db.create_api_key(auth.user_id, &req.key_name).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;
    Ok(Json(json!({ "apiKey": key })))
}

pub async fn delete_api_key(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let deleted = state.db.delete_api_key(auth.user_id, id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;
    Ok(Json(json!({ "deleted": deleted })))
}

#[derive(Deserialize)]
pub struct ToggleRequest {
    #[serde(rename = "isActive")]
    is_active: bool,
}

pub async fn toggle_api_key(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ToggleRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let toggled = state
        .db
        .toggle_api_key(auth.user_id, id, req.is_active)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;
    Ok(Json(json!({ "toggled": toggled })))
}

// --- Credentials ---

pub async fn get_credentials(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let creds = state.db.get_credentials(auth.user_id, None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;
    Ok(Json(json!({ "credentials": creds })))
}

#[derive(Deserialize)]
pub struct CreateCredentialRequest {
    #[serde(rename = "credentialName")]
    credential_name: String,
    #[serde(rename = "credentialType")]
    credential_type: String,
    #[serde(rename = "credentialValue")]
    credential_value: String,
    description: Option<String>,
}

pub async fn create_credential(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateCredentialRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let cred = state
        .db
        .create_credential(
            auth.user_id,
            &req.credential_name,
            &req.credential_type,
            &req.credential_value,
            req.description.as_deref(),
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;
    Ok(Json(json!({ "credential": cred })))
}

pub async fn delete_credential(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let deleted = state.db.delete_credential(auth.user_id, id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;
    Ok(Json(json!({ "deleted": deleted })))
}

pub async fn toggle_credential(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ToggleRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let toggled = state
        .db
        .toggle_credential(auth.user_id, id, req.is_active)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;
    Ok(Json(json!({ "toggled": toggled })))
}

// --- Session Names ---

#[derive(Deserialize)]
pub struct SetSessionNameRequest {
    #[serde(rename = "customName")]
    custom_name: String,
}

pub async fn set_session_name(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path((provider, session_id)): Path<(String, String)>,
    Json(req): Json<SetSessionNameRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    state
        .db
        .set_session_name(&session_id, &provider, &req.custom_name)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;
    Ok(Json(json!({ "success": true })))
}

pub async fn get_session_name(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path((provider, session_id)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let name = state
        .db
        .get_session_name(&session_id, &provider)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;
    Ok(Json(json!({ "customName": name })))
}

pub async fn delete_session_name(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path((provider, session_id)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let deleted = state
        .db
        .delete_session_name(&session_id, &provider)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;
    Ok(Json(json!({ "deleted": deleted })))
}
