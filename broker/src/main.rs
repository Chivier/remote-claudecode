mod protocol;
mod pty;
mod session;
mod updater;

use clap::Parser;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use protocol::{BrokerRequest, BrokerResponse};
use session::SessionManager;

#[derive(Parser)]
#[command(name = "cloudcli-broker", version, about = "CloudCLI Broker - CLI process manager")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "9999")]
    port: u16,

    /// Bind address
    #[arg(short, long, default_value = "127.0.0.1")]
    bind: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cloudcli_broker=info".into()),
        )
        .init();

    let addr = format!("{}:{}", args.bind, args.port);
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind broker address");

    tracing::info!("CloudCLI Broker listening on {}", addr);

    while let Ok((stream, peer)) = listener.accept().await {
        tracing::info!("New connection from {}", peer);
        tokio::spawn(async move {
            match accept_async(stream).await {
                Ok(ws_stream) => {
                    handle_connection(ws_stream).await;
                }
                Err(e) => {
                    tracing::error!("WebSocket handshake failed: {}", e);
                }
            }
        });
    }
}

async fn handle_connection(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
) {
    let (sender, mut receiver) = ws_stream.split();
    let sender = Arc::new(Mutex::new(sender));
    let session_manager = Arc::new(SessionManager::new());

    while let Some(msg) = receiver.next().await {
        let msg: String = match msg {
            Ok(Message::Text(text)) => text.into(),
            Ok(Message::Close(_)) => {
                tracing::info!("Connection closed");
                break;
            }
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
            _ => continue,
        };

        let request: BrokerRequest = match serde_json::from_str(&msg) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to parse request: {}", e);
                let error_response = BrokerResponse::Error {
                    session_id: None,
                    error: format!("Invalid request: {}", e),
                };
                let json = serde_json::to_string(&error_response).unwrap();
                let mut s = sender.lock().await;
                let _ = s.send(Message::Text(json.into())).await;
                continue;
            }
        };

        let sender_clone = sender.clone();
        let sm = session_manager.clone();

        match request {
            BrokerRequest::Command {
                session_id,
                provider,
                command,
                options,
            } => {
                tokio::spawn(async move {
                    sm.handle_command(session_id, provider, command, options, sender_clone)
                        .await;
                });
            }
            BrokerRequest::Abort { session_id } => {
                session_manager.abort_session(&session_id).await;
            }
            BrokerRequest::Status { session_id } => {
                let is_active = session_manager.is_active(&session_id);
                let response = if is_active {
                    serde_json::json!({"type": "status", "sessionId": session_id, "active": true})
                } else {
                    serde_json::json!({"type": "status", "sessionId": session_id, "active": false})
                };
                let json = serde_json::to_string(&response).unwrap();
                let mut s = sender.lock().await;
                let _ = s.send(Message::Text(json.into())).await;
            }
            BrokerRequest::Ping => {
                let response = BrokerResponse::Pong {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    cli_versions: updater::get_cli_versions().await,
                };
                let json = serde_json::to_string(&response).unwrap();
                let mut s = sender.lock().await;
                let _ = s.send(Message::Text(json.into())).await;
            }
            BrokerRequest::UpdateCli { provider } => {
                let sender_clone = sender.clone();
                tokio::spawn(async move {
                    let result = updater::update_cli(&provider).await;
                    let response = BrokerResponse::UpdateResult {
                        provider,
                        success: result.is_ok(),
                        new_version: result.unwrap_or_else(|e| e.to_string()),
                    };
                    let json = serde_json::to_string(&response).unwrap();
                    let mut s = sender_clone.lock().await;
                    let _ = s.send(Message::Text(json.into())).await;
                });
            }
            BrokerRequest::ShellInit {
                session_id,
                cols,
                rows,
                cwd,
            } => {
                let sender_clone = sender.clone();
                let sm = session_manager.clone();
                tokio::spawn(async move {
                    sm.init_shell(session_id, cols, rows, cwd, sender_clone)
                        .await;
                });
            }
            BrokerRequest::ShellInput { session_id, data } => {
                session_manager.shell_input(&session_id, &data).await;
            }
            BrokerRequest::ShellResize {
                session_id,
                cols,
                rows,
            } => {
                session_manager.shell_resize(&session_id, cols, rows);
            }
            BrokerRequest::PermissionResponse {
                request_id,
                approved,
            } => {
                session_manager
                    .handle_permission_response(&request_id, approved)
                    .await;
            }
        }
    }

    // Cleanup all sessions for this connection
    session_manager.cleanup_all().await;
    tracing::info!("Connection handler finished");
}
