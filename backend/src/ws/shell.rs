use axum::extract::ws::{Message, WebSocket};
use futures::StreamExt;
use serde::Deserialize;

use super::writer::WsWriter;
use crate::auth::middleware::{authenticate_websocket, AppState};
use crate::remote::dispatcher::ConnectionDispatcher;
use crate::ws::protocol::{BrokerRequest, OutboundMessage};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ShellMessage {
    #[serde(rename = "auth")]
    Auth { token: Option<String> },
    #[serde(rename = "init")]
    Init {
        cols: Option<u16>,
        rows: Option<u16>,
        cwd: Option<String>,
        #[serde(rename = "serverId")]
        server_id: Option<String>,
    },
    #[serde(rename = "input")]
    Input { data: String },
    #[serde(rename = "resize")]
    Resize { cols: u16, rows: u16 },
}

pub async fn handle_shell_connection(ws: WebSocket, state: AppState, dispatcher: ConnectionDispatcher) {
    let (sender, mut receiver) = ws.split();
    let writer = WsWriter::new(sender);

    // Authenticate
    let auth_user = match receiver.next().await {
        Some(Ok(Message::Text(text))) => {
            let text_str = text.to_string();
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&text_str);
            match parsed {
                Ok(val) => {
                    let token = val.get("token").and_then(|t| t.as_str());
                    authenticate_websocket(
                        token,
                        &state.db,
                        &state.jwt_secret,
                        state.config.is_platform,
                    )
                }
                Err(_) => None,
            }
        }
        _ => None,
    };

    let user = match auth_user {
        Some(u) => u,
        None => {
            let _ = writer
                .send(&OutboundMessage::Error {
                    error: "Authentication failed".to_string(),
                })
                .await;
            writer.close().await;
            return;
        }
    };

    tracing::info!("Shell WebSocket authenticated for user: {}", user.username);

    let session_id = uuid::Uuid::new_v4().to_string();
    let mut server_id = "local".to_string();

    // Message loop
    while let Some(msg) = receiver.next().await {
        let text = match msg {
            Ok(Message::Text(t)) => t.to_string(),
            Ok(Message::Close(_)) => break,
            Err(e) => {
                tracing::error!("Shell WebSocket error: {}", e);
                break;
            }
            _ => continue,
        };

        let shell_msg: ShellMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Failed to parse shell message: {}", e);
                continue;
            }
        };

        match shell_msg {
            ShellMessage::Auth { .. } => {
                // Already authenticated above
            }
            ShellMessage::Init {
                cols,
                rows,
                cwd,
                server_id: sid,
            } => {
                if let Some(sid) = sid {
                    server_id = sid;
                }
                let req = BrokerRequest::ShellInit {
                    session_id: session_id.clone(),
                    cols: cols.unwrap_or(80),
                    rows: rows.unwrap_or(24),
                    cwd: cwd.unwrap_or_else(|| ".".to_string()),
                };
                if let Err(e) = dispatcher
                    .dispatch_broker_request(&server_id, req, &writer)
                    .await
                {
                    tracing::error!("Shell init dispatch error: {}", e);
                }
            }
            ShellMessage::Input { data } => {
                let req = BrokerRequest::ShellInput {
                    session_id: session_id.clone(),
                    data,
                };
                if let Err(e) = dispatcher
                    .dispatch_broker_request(&server_id, req, &writer)
                    .await
                {
                    tracing::error!("Shell input dispatch error: {}", e);
                }
            }
            ShellMessage::Resize { cols, rows } => {
                let req = BrokerRequest::ShellResize {
                    session_id: session_id.clone(),
                    cols,
                    rows,
                };
                if let Err(e) = dispatcher
                    .dispatch_broker_request(&server_id, req, &writer)
                    .await
                {
                    tracing::error!("Shell resize dispatch error: {}", e);
                }
            }
        }
    }

    tracing::info!("Shell WebSocket closed for user: {}", user.username);
}
