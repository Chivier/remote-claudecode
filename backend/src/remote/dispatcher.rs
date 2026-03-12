use dashmap::DashMap;
use std::sync::Arc;

use super::broker_client::BrokerClient;
use super::tunnel::TunnelManager;
use crate::db::Database;
use crate::ws::protocol::{BrokerRequest, CommandOptions, InboundMessage};
use crate::ws::writer::WsWriter;

/// Unified dispatcher that routes messages to the correct broker
/// (local or remote, same code path)
#[derive(Clone)]
pub struct ConnectionDispatcher {
    tunnel_manager: Arc<TunnelManager>,
    broker_connections: Arc<DashMap<String, Arc<BrokerClient>>>,
    local_broker_port: u16,
    db: Database,
}

impl ConnectionDispatcher {
    pub fn new(
        tunnel_manager: Arc<TunnelManager>,
        local_broker_port: u16,
        db: Database,
    ) -> Self {
        Self {
            tunnel_manager,
            broker_connections: Arc::new(DashMap::new()),
            local_broker_port,
            db,
        }
    }

    /// Dispatch an inbound chat message to the appropriate broker
    pub async fn dispatch(
        &self,
        server_id: &str,
        message: InboundMessage,
        writer: &WsWriter,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let broker_port = self.resolve_broker_port(server_id).await?;

        let broker = self
            .get_or_connect_broker(server_id, broker_port, writer)
            .await?;

        let broker_request = self.translate_to_broker(message)?;
        broker.send(&broker_request).await?;

        // Reset idle timer for remote servers
        self.tunnel_manager.reset_idle_timer(server_id);

        Ok(())
    }

    /// Dispatch a raw BrokerRequest (used by shell handler)
    pub async fn dispatch_broker_request(
        &self,
        server_id: &str,
        request: BrokerRequest,
        writer: &WsWriter,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let broker_port = self.resolve_broker_port(server_id).await?;

        let broker = self
            .get_or_connect_broker(server_id, broker_port, writer)
            .await?;

        broker.send(&request).await?;
        self.tunnel_manager.reset_idle_timer(server_id);

        Ok(())
    }

    async fn resolve_broker_port(
        &self,
        server_id: &str,
    ) -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
        let server = self
            .db
            .get_server(server_id)?
            .ok_or_else(|| format!("Server {} not found", server_id))?;

        if server.is_local {
            Ok(self.local_broker_port)
        } else {
            let port = self.tunnel_manager.ensure_tunnel(server_id).await?;
            Ok(port)
        }
    }

    async fn get_or_connect_broker(
        &self,
        server_id: &str,
        port: u16,
        writer: &WsWriter,
    ) -> Result<Arc<BrokerClient>, Box<dyn std::error::Error + Send + Sync>> {
        // Check for existing connection
        if let Some(client) = self.broker_connections.get(server_id) {
            return Ok(client.clone());
        }

        // Create new connection
        let client = BrokerClient::connect(port, writer.clone(), server_id.to_string()).await?;
        let client = Arc::new(client);
        self.broker_connections
            .insert(server_id.to_string(), client.clone());

        Ok(client)
    }

    fn translate_to_broker(
        &self,
        message: InboundMessage,
    ) -> Result<BrokerRequest, Box<dyn std::error::Error + Send + Sync>> {
        match message {
            InboundMessage::ClaudeCommand { command, options } => Ok(BrokerRequest::Command {
                session_id: options.session_id.clone().unwrap_or_default(),
                provider: "claude".to_string(),
                command,
                options: serde_json::to_value(&options)?,
            }),
            InboundMessage::CursorCommand { command, options } => Ok(BrokerRequest::Command {
                session_id: options.session_id.clone().unwrap_or_default(),
                provider: "cursor".to_string(),
                command,
                options: serde_json::to_value(&options)?,
            }),
            InboundMessage::CodexCommand { command, options } => Ok(BrokerRequest::Command {
                session_id: options.session_id.clone().unwrap_or_default(),
                provider: "codex".to_string(),
                command,
                options: serde_json::to_value(&options)?,
            }),
            InboundMessage::GeminiCommand { command, options } => Ok(BrokerRequest::Command {
                session_id: options.session_id.clone().unwrap_or_default(),
                provider: "gemini".to_string(),
                command,
                options: serde_json::to_value(&options)?,
            }),
            InboundMessage::AbortSession {
                session_id,
                provider,
            } => Ok(BrokerRequest::Abort { session_id }),
            InboundMessage::CheckSessionStatus {
                session_id,
                provider,
            } => Ok(BrokerRequest::Status { session_id }),
            InboundMessage::PermissionResponse {
                request_id,
                approved,
            } => Ok(BrokerRequest::PermissionResponse {
                request_id,
                approved,
            }),
        }
    }

    /// Remove a broker connection (e.g., on disconnect)
    pub fn remove_broker(&self, server_id: &str) {
        self.broker_connections.remove(server_id);
    }
}
