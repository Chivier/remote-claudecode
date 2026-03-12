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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inbound_claude_command_deserialize() {
        let json = r#"{
            "type": "claude-command",
            "command": "hello world",
            "projectPath": "/home/user/project",
            "sessionId": "sess-1",
            "serverId": "local"
        }"#;
        let msg: InboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            InboundMessage::ClaudeCommand { command, options } => {
                assert_eq!(command, "hello world");
                assert_eq!(options.project_path.unwrap(), "/home/user/project");
                assert_eq!(options.session_id.unwrap(), "sess-1");
                assert_eq!(options.server_id.unwrap(), "local");
            }
            _ => panic!("expected ClaudeCommand"),
        }
    }

    #[test]
    fn test_inbound_abort_session() {
        let json = r#"{"type": "abort-session", "sessionId": "s1", "provider": "claude"}"#;
        let msg: InboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            InboundMessage::AbortSession { session_id, provider } => {
                assert_eq!(session_id, "s1");
                assert_eq!(provider, "claude");
            }
            _ => panic!("expected AbortSession"),
        }
    }

    #[test]
    fn test_inbound_permission_response() {
        let json = r#"{"type": "permission-response", "requestId": "req-1", "approved": true}"#;
        let msg: InboundMessage = serde_json::from_str(json).unwrap();
        match msg {
            InboundMessage::PermissionResponse { request_id, approved } => {
                assert_eq!(request_id, "req-1");
                assert!(approved);
            }
            _ => panic!("expected PermissionResponse"),
        }
    }

    #[test]
    fn test_server_id_extraction() {
        let json = r#"{"type": "claude-command", "command": "test", "serverId": "remote-1"}"#;
        let msg: InboundMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.server_id(), Some("remote-1"));

        let json2 = r#"{"type": "abort-session", "sessionId": "s1", "provider": "claude"}"#;
        let msg2: InboundMessage = serde_json::from_str(json2).unwrap();
        assert_eq!(msg2.server_id(), None);
    }

    #[test]
    fn test_provider_name() {
        let cmds = vec![
            (r#"{"type": "claude-command", "command": "x"}"#, "claude"),
            (r#"{"type": "cursor-command", "command": "x"}"#, "cursor"),
            (r#"{"type": "codex-command", "command": "x"}"#, "codex"),
            (r#"{"type": "gemini-command", "command": "x"}"#, "gemini"),
        ];
        for (json, expected) in cmds {
            let msg: InboundMessage = serde_json::from_str(json).unwrap();
            assert_eq!(msg.provider_name(), expected);
        }
    }

    #[test]
    fn test_outbound_session_created_serialize() {
        let msg = OutboundMessage::SessionCreated {
            session_id: "sess-123".to_string(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "session-created");
        assert_eq!(json["sessionId"], "sess-123");
    }

    #[test]
    fn test_outbound_claude_complete_serialize() {
        let msg = OutboundMessage::ClaudeComplete {
            session_id: "s1".to_string(),
            exit_code: 0,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "claude-complete");
        assert_eq!(json["exitCode"], 0);
    }

    #[test]
    fn test_broker_request_roundtrip() {
        let req = BrokerRequest::Command {
            session_id: "s1".to_string(),
            provider: "claude".to_string(),
            command: "hello".to_string(),
            options: serde_json::json!({"model": "opus"}),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: BrokerRequest = serde_json::from_str(&json).unwrap();
        match parsed {
            BrokerRequest::Command { session_id, provider, command, .. } => {
                assert_eq!(session_id, "s1");
                assert_eq!(provider, "claude");
                assert_eq!(command, "hello");
            }
            _ => panic!("expected Command"),
        }
    }

    #[test]
    fn test_broker_response_roundtrip() {
        let resp = BrokerResponse::Complete {
            session_id: "s1".to_string(),
            exit_code: 0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: BrokerResponse = serde_json::from_str(&json).unwrap();
        match parsed {
            BrokerResponse::Complete { session_id, exit_code } => {
                assert_eq!(session_id, "s1");
                assert_eq!(exit_code, 0);
            }
            _ => panic!("expected Complete"),
        }
    }

    #[test]
    fn test_broker_ping_pong() {
        let ping = BrokerRequest::Ping;
        let json = serde_json::to_string(&ping).unwrap();
        assert!(json.contains("ping"));

        let pong = BrokerResponse::Pong {
            version: "0.1.0".to_string(),
            cli_versions: std::collections::HashMap::new(),
        };
        let json = serde_json::to_string(&pong).unwrap();
        let parsed: BrokerResponse = serde_json::from_str(&json).unwrap();
        match parsed {
            BrokerResponse::Pong { version, .. } => assert_eq!(version, "0.1.0"),
            _ => panic!("expected Pong"),
        }
    }
}
