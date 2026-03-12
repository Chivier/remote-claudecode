use axum::{http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::process::Command;

use crate::auth::middleware::AuthUser;

#[derive(Deserialize)]
pub struct GitRequest {
    #[serde(rename = "projectPath")]
    project_path: String,
}

#[derive(Deserialize)]
pub struct GitCheckoutRequest {
    #[serde(rename = "projectPath")]
    project_path: String,
    branch: String,
}

#[derive(Deserialize)]
pub struct GitCommitRequest {
    #[serde(rename = "projectPath")]
    project_path: String,
    message: String,
    files: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct GitPushRequest {
    #[serde(rename = "projectPath")]
    project_path: String,
    remote: Option<String>,
    branch: Option<String>,
}

#[derive(Deserialize)]
pub struct GitStashRequest {
    #[serde(rename = "projectPath")]
    project_path: String,
    action: Option<String>, // "save", "pop", "list", "apply", "drop"
}

async fn run_git_command(
    cwd: &str,
    args: &[&str],
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to execute git: {}", e)})),
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(Json(json!({
            "success": true,
            "output": stdout,
            "stderr": stderr,
        })))
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": stderr,
                "output": stdout,
            })),
        ))
    }
}

pub async fn git_status(
    _auth: AuthUser,
    Json(req): Json<GitRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    run_git_command(&req.project_path, &["status", "--porcelain", "-b"]).await
}

pub async fn git_log(
    _auth: AuthUser,
    Json(req): Json<GitRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    run_git_command(
        &req.project_path,
        &["log", "--oneline", "--graph", "--decorate", "-50"],
    )
    .await
}

pub async fn git_diff(
    _auth: AuthUser,
    Json(req): Json<GitRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    run_git_command(&req.project_path, &["diff"]).await
}

pub async fn git_branch(
    _auth: AuthUser,
    Json(req): Json<GitRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    run_git_command(&req.project_path, &["branch", "-a"]).await
}

pub async fn git_checkout(
    _auth: AuthUser,
    Json(req): Json<GitCheckoutRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    run_git_command(&req.project_path, &["checkout", &req.branch]).await
}

pub async fn git_commit(
    _auth: AuthUser,
    Json(req): Json<GitCommitRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Stage files if specified
    if let Some(ref files) = req.files {
        for file in files {
            run_git_command(&req.project_path, &["add", file]).await?;
        }
    }

    run_git_command(&req.project_path, &["commit", "-m", &req.message]).await
}

pub async fn git_push(
    _auth: AuthUser,
    Json(req): Json<GitPushRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let remote = req.remote.as_deref().unwrap_or("origin");
    let mut args = vec!["push", remote];
    if let Some(ref branch) = req.branch {
        args.push(branch.as_str());
    }
    run_git_command(&req.project_path, &args).await
}

pub async fn git_pull(
    _auth: AuthUser,
    Json(req): Json<GitRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    run_git_command(&req.project_path, &["pull"]).await
}

pub async fn git_stash(
    _auth: AuthUser,
    Json(req): Json<GitStashRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let action = req.action.as_deref().unwrap_or("save");
    run_git_command(&req.project_path, &["stash", action]).await
}
