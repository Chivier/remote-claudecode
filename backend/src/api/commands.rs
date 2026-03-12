use axum::{http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::process::Command;

use crate::auth::middleware::AuthUser;

#[derive(Deserialize)]
pub struct CommandRequest {
    command: String,
    cwd: Option<String>,
}

pub async fn execute_command(
    _auth: AuthUser,
    Json(req): Json<CommandRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let cwd = req.cwd.as_deref().unwrap_or(".");

    let output = Command::new("sh")
        .arg("-c")
        .arg(&req.command)
        .current_dir(cwd)
        .output()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to execute command: {}", e)})),
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(Json(json!({
        "success": output.status.success(),
        "exitCode": output.status.code(),
        "stdout": stdout,
        "stderr": stderr,
    })))
}
