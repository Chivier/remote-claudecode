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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_command_request() {
        let json = r#"{"type":"command","session_id":"s1","provider":"claude","command":"hello","options":{}}"#;
        let req: BrokerRequest = serde_json::from_str(json).unwrap();
        match req {
            BrokerRequest::Command { session_id, provider, command, .. } => {
                assert_eq!(session_id, "s1");
                assert_eq!(provider, "claude");
                assert_eq!(command, "hello");
            }
            _ => panic!("expected Command"),
        }
    }

    #[test]
    fn test_deserialize_ping() {
        let json = r#"{"type":"ping"}"#;
        let req: BrokerRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, BrokerRequest::Ping));
    }

    #[test]
    fn test_deserialize_shell_init() {
        let json = r#"{"type":"shell-init","session_id":"shell-1","cols":80,"rows":24,"cwd":"/home/user"}"#;
        let req: BrokerRequest = serde_json::from_str(json).unwrap();
        match req {
            BrokerRequest::ShellInit { session_id, cols, rows, cwd } => {
                assert_eq!(session_id, "shell-1");
                assert_eq!(cols, 80);
                assert_eq!(rows, 24);
                assert_eq!(cwd, "/home/user");
            }
            _ => panic!("expected ShellInit"),
        }
    }

    #[test]
    fn test_serialize_pong() {
        let mut cli_versions = HashMap::new();
        cli_versions.insert("claude".to_string(), "1.0.0".to_string());
        let resp = BrokerResponse::Pong {
            version: "0.1.0".to_string(),
            cli_versions,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["type"], "pong");
        assert_eq!(json["version"], "0.1.0");
        assert_eq!(json["cli_versions"]["claude"], "1.0.0");
    }

    #[test]
    fn test_serialize_error() {
        let resp = BrokerResponse::Error {
            session_id: Some("s1".to_string()),
            error: "something went wrong".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["type"], "error");
        assert_eq!(json["error"], "something went wrong");
    }

    #[test]
    fn test_serialize_session_created() {
        let resp = BrokerResponse::SessionCreated {
            session_id: "req-id".to_string(),
            actual_session_id: "actual-id".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["type"], "session-created");
        assert_eq!(json["actual_session_id"], "actual-id");
    }

    #[test]
    fn test_deserialize_permission_response() {
        let json = r#"{"type":"permission-response","request_id":"req-1","approved":false}"#;
        let req: BrokerRequest = serde_json::from_str(json).unwrap();
        match req {
            BrokerRequest::PermissionResponse { request_id, approved } => {
                assert_eq!(request_id, "req-1");
                assert!(!approved);
            }
            _ => panic!("expected PermissionResponse"),
        }
    }
}
