use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use futures::SinkExt;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::protocol::OutboundMessage;

/// Thread-safe WebSocket writer that serializes outbound messages as JSON
#[derive(Clone)]
pub struct WsWriter {
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
}

impl WsWriter {
    pub fn new(sender: SplitSink<WebSocket, Message>) -> Self {
        Self {
            sender: Arc::new(Mutex::new(sender)),
        }
    }

    pub async fn send(&self, msg: &OutboundMessage) -> Result<(), axum::Error> {
        let json = serde_json::to_string(msg).map_err(|e| {
            tracing::error!("Failed to serialize outbound message: {}", e);
            axum::Error::new(e)
        })?;
        let mut sender = self.sender.lock().await;
        sender.send(Message::Text(json.into())).await.map_err(|e| {
            tracing::error!("Failed to send WebSocket message: {}", e);
            axum::Error::new(e)
        })
    }

    pub async fn send_json(&self, value: &serde_json::Value) -> Result<(), axum::Error> {
        let json = serde_json::to_string(value).map_err(|e| axum::Error::new(e))?;
        let mut sender = self.sender.lock().await;
        sender.send(Message::Text(json.into())).await.map_err(|e| axum::Error::new(e))
    }

    pub async fn send_raw(&self, text: String) -> Result<(), axum::Error> {
        let mut sender = self.sender.lock().await;
        sender.send(Message::Text(text.into())).await.map_err(|e| axum::Error::new(e))
    }

    pub async fn close(&self) {
        let mut sender = self.sender.lock().await;
        let _ = sender.send(Message::Close(None)).await;
    }
}
