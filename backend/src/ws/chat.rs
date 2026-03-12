use axum::extract::ws::{Message, WebSocket};
use futures::StreamExt;

use super::protocol::{InboundMessage, OutboundMessage};
use super::writer::WsWriter;
use crate::auth::middleware::{authenticate_websocket, AppState};
use crate::remote::dispatcher::ConnectionDispatcher;

pub async fn handle_chat_connection(ws: WebSocket, state: AppState, dispatcher: ConnectionDispatcher) {
    let (sender, mut receiver) = ws.split();
    let writer = WsWriter::new(sender);

    // First message should be auth
    let auth_user = match receiver.next().await {
        Some(Ok(Message::Text(text))) => {
            // Parse auth message: {"type": "auth", "token": "..."}
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

    tracing::info!("Chat WebSocket authenticated for user: {}", user.username);

    // Send auth success
    let _ = writer
        .send_json(&serde_json::json!({
            "type": "auth-success",
            "userId": user.user_id,
            "username": user.username
        }))
        .await;

    // Message loop
    while let Some(msg) = receiver.next().await {
        let text = match msg {
            Ok(Message::Text(t)) => t.to_string(),
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(_)) => continue, // pong handled by axum
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
            _ => continue,
        };

        let inbound: InboundMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Failed to parse inbound message: {}", e);
                let _ = writer
                    .send(&OutboundMessage::Error {
                        error: format!("Invalid message format: {}", e),
                    })
                    .await;
                continue;
            }
        };

        let server_id = inbound.server_id().unwrap_or("local").to_string();

        // Dispatch through the unified dispatcher
        if let Err(e) = dispatcher.dispatch(&server_id, inbound, &writer).await {
            tracing::error!("Dispatch error: {}", e);
            let _ = writer
                .send(&OutboundMessage::Error {
                    error: format!("Dispatch error: {}", e),
                })
                .await;
        }
    }

    tracing::info!("Chat WebSocket closed for user: {}", user.username);
}
