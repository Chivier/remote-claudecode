use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Messages from Backend to Broker
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum BrokerRequest {
    #[serde(rename = "command")]
    Command {
        session_id: String,
        provider: String,
        command: String,
        options: Value,
    },
    #[serde(rename = "abort")]
    Abort { session_id: String },
    #[serde(rename = "status")]
    Status { session_id: String },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "update-cli")]
    UpdateCli { provider: String },
    #[serde(rename = "shell-init")]
    ShellInit {
        session_id: String,
        cols: u16,
        rows: u16,
        cwd: String,
    },
    #[serde(rename = "shell-input")]
    ShellInput { session_id: String, data: String },
    #[serde(rename = "shell-resize")]
    ShellResize {
        session_id: String,
        cols: u16,
        rows: u16,
    },
    #[serde(rename = "permission-response")]
    PermissionResponse {
        request_id: String,
        approved: bool,
    },
}

/// Messages from Broker to Backend
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum BrokerResponse {
    #[serde(rename = "session-created")]
    SessionCreated {
        session_id: String,
        actual_session_id: String,
    },
    #[serde(rename = "provider-message")]
    ProviderMessage {
        session_id: String,
        provider: String,
        data: Value,
    },
    #[serde(rename = "complete")]
    Complete {
        session_id: String,
        exit_code: i32,
    },
    #[serde(rename = "error")]
    Error {
        session_id: Option<String>,
        error: String,
    },
    #[serde(rename = "pong")]
    Pong {
        version: String,
        cli_versions: HashMap<String, String>,
    },
    #[serde(rename = "shell-output")]
    ShellOutput { session_id: String, data: String },
    #[serde(rename = "shell-exit")]
    ShellExit {
        session_id: String,
        exit_code: i32,
    },
    #[serde(rename = "update-result")]
    UpdateResult {
        provider: String,
        success: bool,
        new_version: String,
    },
    #[serde(rename = "permission-request")]
    PermissionRequest {
        session_id: String,
        request_id: String,
        tool_name: String,
        params: Value,
    },
}
