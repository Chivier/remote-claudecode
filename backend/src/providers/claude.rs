use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::Provider;
use crate::ws::protocol::{CommandOptions, OutboundMessage};
use crate::ws::writer::WsWriter;

pub struct ClaudeProvider {
    cli_path: String,
    sessions: DashMap<String, Arc<Mutex<Child>>>,
}

impl ClaudeProvider {
    pub fn new(cli_path: &str) -> Self {
        Self {
            cli_path: cli_path.to_string(),
            sessions: DashMap::new(),
        }
    }

    fn build_args(&self, command: &str, options: &CommandOptions) -> Vec<String> {
        let mut args = vec![
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];

        if let Some(ref model) = options.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }

        if let Some(ref session_id) = options.session_id {
            if options.resume.unwrap_or(false) || options.continue_conversation.unwrap_or(false) {
                args.push("--resume".to_string());
                args.push(session_id.clone());
            }
        }

        if let Some(ref system_prompt) = options.system_prompt {
            args.push("--system-prompt".to_string());
            args.push(system_prompt.clone());
        }

        if let Some(ref append) = options.append_system_prompt {
            args.push("--append-system-prompt".to_string());
            args.push(append.clone());
        }

        if let Some(max_turns) = options.max_turns {
            args.push("--max-turns".to_string());
            args.push(max_turns.to_string());
        }

        if let Some(ref mode) = options.permission_mode {
            args.push("--permission-mode".to_string());
            args.push(mode.clone());
        }

        if let Some(ref tools) = options.allowed_tools {
            for tool in tools {
                args.push("--allowedTools".to_string());
                args.push(tool.clone());
            }
        }

        // The prompt itself
        args.push("--print".to_string());
        args.push(command.to_string());

        args
    }
}

#[async_trait]
impl Provider for ClaudeProvider {
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

        // Notify session created
        let _ = writer
            .send(&OutboundMessage::SessionCreated {
                session_id: session_id.clone(),
            })
            .await;

        let args = self.build_args(command, options);
        let cwd = options
            .cwd
            .as_deref()
            .or(options.project_path.as_deref())
            .unwrap_or(".");

        let mut child = Command::new(&self.cli_path)
            .args(&args)
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        self.sessions
            .insert(session_id.clone(), Arc::new(Mutex::new(child)));

        let writer_clone = writer.clone();
        let sid = session_id.clone();

        // Spawn stderr reader
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!("[claude stderr] {}", line);
            }
        });

        // Read stdout line by line (JSON stream)
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(data) => {
                    let _ = writer_clone
                        .send(&OutboundMessage::ClaudeResponse {
                            data,
                            session_id: sid.clone(),
                        })
                        .await;
                }
                Err(e) => {
                    tracing::warn!("Failed to parse claude output: {} — line: {}", e, line);
                }
            }
        }

        // Wait for process to exit
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
            .send(&OutboundMessage::ClaudeComplete {
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
        "claude"
    }
}
