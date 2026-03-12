pub mod claude;
pub mod codex;
pub mod cursor;
pub mod gemini;

use async_trait::async_trait;

use crate::ws::protocol::CommandOptions;
use crate::ws::writer::WsWriter;

#[async_trait]
pub trait Provider: Send + Sync {
    /// Execute a command with streaming output to the WebSocket writer
    async fn query(
        &self,
        command: &str,
        options: &CommandOptions,
        writer: &WsWriter,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Abort an active session
    async fn abort(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Check if a session is currently active
    fn is_active(&self, session_id: &str) -> bool;

    /// Get the provider name
    fn provider_name(&self) -> &str;
}
