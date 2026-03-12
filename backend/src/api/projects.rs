use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::middleware::{AppState, AuthUser};
use crate::utils::projects as project_utils;

pub async fn list_projects(
    _auth: AuthUser,
    State(_state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let base_dirs = vec![
        format!("{}/Projects", home),
        format!("{}/projects", home),
        format!("{}/Documents/Projects", home),
        format!("{}/repos", home),
        format!("{}/workspace", home),
        format!("{}/dev", home),
        format!("{}/src", home),
        format!("{}/code", home),
    ];
    let base_refs: Vec<&str> = base_dirs.iter().map(|s| s.as_str()).collect();

    let projects = project_utils::discover_projects(&base_refs).await;

    Ok(Json(json!({ "projects": projects })))
}

#[derive(Deserialize)]
pub struct CreateWorkspaceRequest {
    path: String,
    #[serde(rename = "initGit")]
    init_git: Option<bool>,
}

pub async fn create_workspace(
    _auth: AuthUser,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match project_utils::create_workspace(&req.path, req.init_git.unwrap_or(true)) {
        Ok(path) => Ok(Json(json!({ "path": path }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )),
    }
}
