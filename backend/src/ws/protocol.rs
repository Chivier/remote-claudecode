use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- Inbound messages (Frontend → Backend) ---

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum InboundMessage {
    #[serde(rename = "claude-command")]
    ClaudeCommand {
        command: String,
        #[serde(flatten)]
        options: CommandOptions,
    },
    #[serde(rename = "cursor-command")]
    CursorCommand {
        command: String,
        #[serde(flatten)]
        options: CommandOptions,
    },
    #[serde(rename = "codex-command")]
    CodexCommand {
        command: String,
        #[serde(flatten)]
        options: CommandOptions,
    },
    #[serde(rename = "gemini-command")]
    GeminiCommand {
        command: String,
        #[serde(flatten)]
        options: CommandOptions,
    },
    #[serde(rename = "abort-session")]
    AbortSession {
        #[serde(rename = "sessionId")]
        session_id: String,
        provider: String,
    },
    #[serde(rename = "check-session-status")]
    CheckSessionStatus {
        #[serde(rename = "sessionId")]
        session_id: String,
        provider: String,
    },
    #[serde(rename = "permission-response")]
    PermissionResponse {
        #[serde(rename = "requestId")]
        request_id: String,
        approved: bool,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandOptions {
    #[serde(rename = "projectPath")]
    pub project_path: Option<String>,
    pub cwd: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    pub resume: Option<bool>,
    pub model: Option<String>,
    #[serde(rename = "serverId")]
    pub server_id: Option<String>,
    #[serde(rename = "maxTurns")]
    pub max_turns: Option<i32>,
    #[serde(rename = "allowedTools")]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: Option<String>,
    #[serde(rename = "appendSystemPrompt")]
    pub append_system_prompt: Option<String>,
    #[serde(rename = "permissionMode")]
    pub permission_mode: Option<String>,
    #[serde(rename = "continueConversation")]
    pub continue_conversation: Option<bool>,
}

impl InboundMessage {
    pub fn server_id(&self) -> Option<&str> {
        match self {
            Self::ClaudeCommand { options, .. }
            | Self::CursorCommand { options, .. }
            | Self::CodexCommand { options, .. }
            | Self::GeminiCommand { options, .. } => options.server_id.as_deref(),
            _ => None,
        }
    }

    pub fn provider_name(&self) -> &str {
        match self {
            Self::ClaudeCommand { .. } => "claude",
            Self::CursorCommand { .. } => "cursor",
            Self::CodexCommand { .. } => "codex",
            Self::GeminiCommand { .. } => "gemini",
            Self::AbortSession { provider, .. } => provider.as_str(),
            Self::CheckSessionStatus { provider, .. } => provider.as_str(),
            Self::PermissionResponse { .. } => "system",
        }
    }
}

// --- Outbound messages (Backend → Frontend) ---

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum OutboundMessage {
    #[serde(rename = "session-created")]
    SessionCreated {
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "claude-response")]
    ClaudeResponse {
        data: Value,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "token-budget")]
    TokenBudget {
        data: Value,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "claude-complete")]
    ClaudeComplete {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "exitCode")]
        exit_code: i32,
    },
    #[serde(rename = "claude-error")]
    ClaudeError {
        error: String,
        #[serde(rename = "sessionId")]
        session_id: Option<String>,
    },
    #[serde(rename = "permission-request")]
    PermissionRequest {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "toolName")]
        tool_name: String,
        params: Value,
    },
    #[serde(rename = "cursor-response")]
    CursorResponse {
        data: Value,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "cursor-complete")]
    CursorComplete {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "exitCode")]
        exit_code: i32,
    },
    #[serde(rename = "codex-response")]
    CodexResponse {
        data: Value,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "codex-complete")]
    CodexComplete {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "exitCode")]
        exit_code: i32,
    },
    #[serde(rename = "gemini-response")]
    GeminiResponse {
        data: Value,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "gemini-complete")]
    GeminiComplete {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "exitCode")]
        exit_code: i32,
    },
    #[serde(rename = "error")]
    Error {
        error: String,
    },
}

// --- Broker protocol messages ---

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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
        cli_versions: std::collections::HashMap<String, String>,
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
