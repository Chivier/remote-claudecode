use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::ws::protocol::{BrokerRequest, BrokerResponse};
use crate::ws::writer::WsWriter;

type WsStream = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

/// Client connection to a Broker instance (local or remote via tunnel)
pub struct BrokerClient {
    sender: Arc<Mutex<futures::stream::SplitSink<WsStream, Message>>>,
    _receiver_task: tokio::task::JoinHandle<()>,
}

impl BrokerClient {
    /// Connect to a broker at the given port on localhost
    pub async fn connect(
        port: u16,
        frontend_writer: WsWriter,
        server_id: String,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("ws://127.0.0.1:{}", port);
        let (ws_stream, _) = connect_async(&url).await?;
        let (sender, mut receiver) = ws_stream.split();

        let sender = Arc::new(Mutex::new(sender));

        // Spawn receiver loop that forwards broker responses to the frontend
        let receiver_task = tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        let text_str: String = text.into();
                        match serde_json::from_str::<BrokerResponse>(&text_str) {
                            Ok(response) => {
                                Self::forward_to_frontend(&response, &frontend_writer, &server_id)
                                    .await;
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse broker response: {}", e);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        tracing::info!("Broker connection closed for server {}", server_id);
                        break;
                    }
                    Err(e) => {
                        tracing::error!("Broker WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(Self {
            sender,
            _receiver_task: receiver_task,
        })
    }

    /// Send a request to the broker
    pub async fn send(&self, request: &BrokerRequest) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json = serde_json::to_string(request)?;
        let mut sender = self.sender.lock().await;
        sender.send(Message::Text(json.into())).await?;
        Ok(())
    }

    /// Forward a broker response to the frontend WebSocket
    async fn forward_to_frontend(
        response: &BrokerResponse,
        writer: &WsWriter,
        _server_id: &str,
    ) {
        use crate::ws::protocol::OutboundMessage;

        let outbound = match response {
            BrokerResponse::SessionCreated {
                session_id,
                actual_session_id,
            } => Some(OutboundMessage::SessionCreated {
                session_id: actual_session_id.clone(),
            }),
            BrokerResponse::ProviderMessage {
                session_id,
                provider,
                data,
            } => match provider.as_str() {
                "claude" => Some(OutboundMessage::ClaudeResponse {
                    data: data.clone(),
                    session_id: session_id.clone(),
                }),
                "cursor" => Some(OutboundMessage::CursorResponse {
                    data: data.clone(),
                    session_id: session_id.clone(),
                }),
                "codex" => Some(OutboundMessage::CodexResponse {
                    data: data.clone(),
                    session_id: session_id.clone(),
                }),
                "gemini" => Some(OutboundMessage::GeminiResponse {
                    data: data.clone(),
                    session_id: session_id.clone(),
                }),
                _ => None,
            },
            BrokerResponse::Complete {
                session_id,
                exit_code,
            } => Some(OutboundMessage::ClaudeComplete {
                session_id: session_id.clone(),
                exit_code: *exit_code,
            }),
            BrokerResponse::Error { session_id, error } => Some(OutboundMessage::ClaudeError {
                error: error.clone(),
                session_id: session_id.clone(),
            }),
            BrokerResponse::PermissionRequest {
                session_id,
                request_id,
                tool_name,
                params,
            } => Some(OutboundMessage::PermissionRequest {
                request_id: request_id.clone(),
                tool_name: tool_name.clone(),
                params: params.clone(),
            }),
            BrokerResponse::ShellOutput { session_id, data } => {
                // Forward as raw JSON for the shell handler
                let _ = writer
                    .send_json(&serde_json::json!({
                        "type": "output",
                        "data": data,
                        "sessionId": session_id
                    }))
                    .await;
                None
            }
            BrokerResponse::ShellExit {
                session_id,
                exit_code,
            } => {
                let _ = writer
                    .send_json(&serde_json::json!({
                        "type": "exit",
                        "exitCode": exit_code,
                        "sessionId": session_id
                    }))
                    .await;
                None
            }
            _ => None,
        };

        if let Some(msg) = outbound {
            let _ = writer.send(&msg).await;
        }
    }
}
