use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::auth::middleware::{AppState, AuthUser};
use crate::db::servers::{CreateServerRequest, UpdateServerRequest};
use crate::remote::deployer;

// We need tunnel_manager accessible from the state
// For now, we'll use a global or pass it through extension

pub async fn list_servers(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let servers = state.db.list_servers(auth.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;
    Ok(Json(json!({ "servers": servers })))
}

pub async fn create_server(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateServerRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let server = state.db.create_server(auth.user_id, &req).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;
    Ok(Json(json!({ "server": server })))
}

pub async fn get_server(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let server = state
        .db
        .get_server(&id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Server not found"})),
            )
        })?;
    Ok(Json(json!({ "server": server })))
}

pub async fn update_server(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateServerRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let updated = state
        .db
        .update_server(&id, auth.user_id, &req)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;

    if !updated {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Server not found or not authorized"})),
        ));
    }

    let server = state.db.get_server(&id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    Ok(Json(json!({ "server": server })))
}

pub async fn delete_server(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let deleted = state.db.delete_server(&id, auth.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    if !deleted {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Cannot delete server (not found or is local)"})),
        ));
    }

    Ok(Json(json!({ "deleted": true })))
}

pub async fn test_connection(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let server = state
        .db
        .get_server(&id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Server not found"})),
            )
        })?;

    if server.is_local {
        return Ok(Json(json!({
            "success": true,
            "message": "Local server is always reachable"
        })));
    }

    match deployer::test_ssh_connection(&server).await {
        Ok(output) => Ok(Json(json!({
            "success": true,
            "output": output
        }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn deploy_broker(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let server = state
        .db
        .get_server(&id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Server not found"})),
            )
        })?;

    if server.is_local {
        return Ok(Json(json!({
            "success": true,
            "message": "Local broker is managed automatically"
        })));
    }

    match deployer::deploy_broker(&server).await {
        Ok(output) => Ok(Json(json!({
            "success": true,
            "output": output
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn get_status(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let server = state
        .db
        .get_server(&id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Server not found"})),
            )
        })?;

    if server.is_local {
        return Ok(Json(json!({
            "status": "connected",
            "isLocal": true
        })));
    }

    // For remote servers, we'd check the tunnel status
    // This requires access to TunnelManager via state extension
    Ok(Json(json!({
        "status": "disconnected",
        "isLocal": false,
        "tunnelPort": server.tunnel_local_port
    })))
}
