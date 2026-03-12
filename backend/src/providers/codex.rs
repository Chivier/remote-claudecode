use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::Provider;
use crate::ws::protocol::{CommandOptions, OutboundMessage};
use crate::ws::writer::WsWriter;

pub struct CodexProvider {
    sessions: DashMap<String, Arc<Mutex<Child>>>,
}

impl CodexProvider {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }
}

#[async_trait]
impl Provider for CodexProvider {
    async fn query(
        &self,
        command: &str,
        options: &CommandOptions,
        writer: &WsWriter,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let session_id = options
            .session_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let _ = writer
            .send(&OutboundMessage::SessionCreated {
                session_id: session_id.clone(),
            })
            .await;

        let cwd = options
            .cwd
            .as_deref()
            .or(options.project_path.as_deref())
            .unwrap_or(".");

        let mut args = vec!["--quiet".to_string()];

        if let Some(ref model) = options.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }

        args.push(command.to_string());

        let mut child = Command::new("codex")
            .args(&args)
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("stdout piped");
        self.sessions
            .insert(session_id.clone(), Arc::new(Mutex::new(child)));

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let writer_clone = writer.clone();
        let sid = session_id.clone();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(data) => {
                    let _ = writer_clone
                        .send(&OutboundMessage::CodexResponse {
                            data,
                            session_id: sid.clone(),
                        })
                        .await;
                }
                Err(_) => {
                    // Non-JSON output, wrap it
                    let _ = writer_clone
                        .send(&OutboundMessage::CodexResponse {
                            data: serde_json::json!({"type": "text", "content": line}),
                            session_id: sid.clone(),
                        })
                        .await;
                }
            }
        }

        let exit_code = if let Some(entry) = self.sessions.get(&session_id) {
            let mut child = entry.lock().await;
            match child.wait().await {
                Ok(status) => status.code().unwrap_or(-1),
                Err(_) => -1,
            }
        } else {
            -1
        };

        self.sessions.remove(&session_id);

        let _ = writer
            .send(&OutboundMessage::CodexComplete {
                session_id,
                exit_code,
            })
            .await;

        Ok(())
    }

    async fn abort(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some((_, child)) = self.sessions.remove(session_id) {
            let mut child = child.lock().await;
            child.kill().await?;
        }
        Ok(())
    }

    fn is_active(&self, session_id: &str) -> bool {
        self.sessions.contains_key(session_id)
    }

    fn provider_name(&self) -> &str {
        "codex"
    }
}
