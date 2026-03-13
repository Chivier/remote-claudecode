use dashmap::DashMap;
use futures::SinkExt;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

use crate::protocol::BrokerResponse;
use crate::pty::PtyManager;

type WsSender = Arc<
    Mutex<
        futures::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
            Message,
        >,
    >,
>;

struct SessionHandle {
    child: Arc<Mutex<Child>>,
    provider: String,
}

pub struct SessionManager {
    sessions: DashMap<String, SessionHandle>,
    pty_manager: PtyManager,
    permission_channels:
        DashMap<String, tokio::sync::oneshot::Sender<bool>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            pty_manager: PtyManager::new(),
            permission_channels: DashMap::new(),
        }
    }

    pub async fn handle_command(
        &self,
        session_id: String,
        provider: String,
        command: String,
        options: serde_json::Value,
        sender: WsSender,
    ) {
        let actual_session_id = if session_id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            session_id.clone()
        };

        // Send session created
        let response = BrokerResponse::SessionCreated {
            session_id: session_id.clone(),
            actual_session_id: actual_session_id.clone(),
        };
        self.send_response(&sender, &response).await;

        // Build CLI command based on provider
        let (program, args) = self.build_cli_command(&provider, &command, &options);

        let cwd = options
            .get("cwd")
            .or(options.get("projectPath"))
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        // Ensure HOME and PATH are set for finding CLI tools
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let path = std::env::var("PATH").unwrap_or_default();
        let enhanced_path = format!("{}/.local/bin:{}/.cargo/bin:/usr/local/bin:/usr/bin:/bin:{}", home, home, path);

        tracing::info!("Spawning {} with args {:?} in {}", program, args, cwd);

        // Spawn the CLI process
        let child = match Command::new(&program)
            .args(&args)
            .current_dir(cwd)
            .env("PATH", &enhanced_path)
            .env("HOME", &home)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                let response = BrokerResponse::Error {
                    session_id: Some(session_id),
                    error: format!("Failed to spawn {}: {}", program, e),
                };
                self.send_response(&sender, &response).await;
                return;
            }
        };

        let child = Arc::new(Mutex::new(child));
        self.sessions.insert(
            actual_session_id.clone(),
            SessionHandle {
                child: child.clone(),
                provider: provider.clone(),
            },
        );

        // Read stdout and stderr
        let (stdout, stderr) = {
            let mut c = child.lock().await;
            (c.stdout.take(), c.stderr.take())
        };

        tracing::info!("Process spawned for session {}", actual_session_id);

        // Drop stdin so claude doesn't wait for input
        {
            let mut c = child.lock().await;
            drop(c.stdin.take());
        }

        // Spawn stderr reader to log errors
        if let Some(stderr) = stderr {
            let sid_err = actual_session_id.clone();
            tokio::spawn(async move {
                tracing::debug!("Started stderr reader for {}", sid_err);
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.trim().is_empty() {
                        tracing::warn!("Session {} stderr: {}", sid_err, line);
                    }
                }
                tracing::debug!("stderr reader done for {}", sid_err);
            });
        }

        if let Some(stdout) = stdout {
            tracing::debug!("Starting stdout reader for {}", actual_session_id);
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let sid = actual_session_id.clone();
            let prov = provider.clone();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                let data = match serde_json::from_str::<serde_json::Value>(&line) {
                    Ok(v) => v,
                    Err(_) => serde_json::json!({"type": "text", "content": line}),
                };

                let response = BrokerResponse::ProviderMessage {
                    session_id: sid.clone(),
                    provider: prov.clone(),
                    data,
                };
                self.send_response(&sender, &response).await;
            }
            tracing::info!("stdout reader done for {}", actual_session_id);
        } else {
            tracing::warn!("No stdout for session {}", actual_session_id);
        }

        // Wait for exit
        let exit_code = {
            let mut c = child.lock().await;
            match c.wait().await {
                Ok(status) => status.code().unwrap_or(-1),
                Err(_) => -1,
            }
        };

        self.sessions.remove(&actual_session_id);

        let response = BrokerResponse::Complete {
            session_id: actual_session_id,
            exit_code,
        };
        self.send_response(&sender, &response).await;
    }

    /// Find the actual path for a CLI tool, checking common locations
    fn find_cli_path(name: &str) -> String {
        // Check common paths where CLI tools are installed
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let candidates = [
            format!("{home}/.local/bin/{name}"),
            format!("{home}/.nvm/versions/node/*/bin/{name}"),  // won't match with glob, but keeping as fallback
            format!("/usr/local/bin/{name}"),
            format!("/usr/bin/{name}"),
            format!("{home}/.cargo/bin/{name}"),
        ];
        for path in &candidates {
            if std::path::Path::new(path).exists() {
                return path.clone();
            }
        }
        // Fall back to bare name (relies on PATH)
        name.to_string()
    }

    fn build_cli_command(
        &self,
        provider: &str,
        command: &str,
        options: &serde_json::Value,
    ) -> (String, Vec<String>) {
        match provider {
            "claude" => {
                let mut args = vec![
                    "--output-format".to_string(),
                    "stream-json".to_string(),
                    "--verbose".to_string(),
                ];

                if let Some(model) = options.get("model").and_then(|v| v.as_str()) {
                    args.push("--model".to_string());
                    args.push(model.to_string());
                }

                if let Some(session_id) = options.get("sessionId").and_then(|v| v.as_str()) {
                    let resume = options
                        .get("resume")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let cont = options
                        .get("continueConversation")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if resume || cont {
                        args.push("--resume".to_string());
                        args.push(session_id.to_string());
                    }
                }

                if let Some(sp) = options.get("systemPrompt").and_then(|v| v.as_str()) {
                    args.push("--system-prompt".to_string());
                    args.push(sp.to_string());
                }

                // Handle permission modes
                if let Some(pm) = options.get("permissionMode").and_then(|v| v.as_str()) {
                    match pm {
                        "dangerouslySkipPermissions" | "bypassPermissions" | "full" => {
                            args.push("--dangerously-skip-permissions".to_string());
                        }
                        "default" | "" => {}
                        other => {
                            // Pass other valid modes via --permission-mode
                            args.push("--permission-mode".to_string());
                            args.push(other.to_string());
                        }
                    }
                }

                args.push("--print".to_string());
                args.push(command.to_string());

                (Self::find_cli_path("claude"), args)
            }
            "codex" => {
                let mut args = vec!["--quiet".to_string()];
                if let Some(model) = options.get("model").and_then(|v| v.as_str()) {
                    args.push("--model".to_string());
                    args.push(model.to_string());
                }
                if let Some(pm) = options.get("permissionMode").and_then(|v| v.as_str()) {
                    match pm {
                        "dangerouslySkipPermissions" | "bypassPermissions" | "full" => {
                            args.push("--full-auto".to_string());
                        }
                        _ => {}
                    }
                }
                args.push(command.to_string());
                (Self::find_cli_path("codex"), args)
            }
            "opencode" => {
                let mut args = vec![];
                if let Some(model) = options.get("model").and_then(|v| v.as_str()) {
                    args.push("--model".to_string());
                    args.push(model.to_string());
                }
                if let Some(pm) = options.get("permissionMode").and_then(|v| v.as_str()) {
                    match pm {
                        "dangerouslySkipPermissions" | "bypassPermissions" | "full" => {
                            args.push("--auto-approve".to_string());
                        }
                        _ => {}
                    }
                }
                args.push("--quiet".to_string());
                args.push(command.to_string());
                (Self::find_cli_path("opencode"), args)
            }
            "gemini" => {
                let mut args = vec![];
                if let Some(model) = options.get("model").and_then(|v| v.as_str()) {
                    args.push("--model".to_string());
                    args.push(model.to_string());
                }
                if let Some(pm) = options.get("permissionMode").and_then(|v| v.as_str()) {
                    match pm {
                        "dangerouslySkipPermissions" | "bypassPermissions" | "full" => {
                            args.push("--sandbox".to_string());
                        }
                        _ => {}
                    }
                }
                args.push(command.to_string());
                (Self::find_cli_path("gemini"), args)
            }
            _ => (provider.to_string(), vec![command.to_string()]),
        }
    }

    pub async fn abort_session(&self, session_id: &str) {
        if let Some((_, handle)) = self.sessions.remove(session_id) {
            let mut child = handle.child.lock().await;
            child.kill().await.ok();
            tracing::info!("Aborted session {}", session_id);
        }
    }

    pub fn is_active(&self, session_id: &str) -> bool {
        self.sessions.contains_key(session_id)
    }

    pub async fn init_shell(
        &self,
        session_id: String,
        cols: u16,
        rows: u16,
        cwd: String,
        sender: WsSender,
    ) {
        match self
            .pty_manager
            .create_session(&session_id, cols, rows, &cwd, sender)
            .await
        {
            Ok(()) => {
                tracing::info!("Shell session {} initialized", session_id);
            }
            Err(e) => {
                tracing::error!("Failed to init shell {}: {}", session_id, e);
            }
        }
    }

    pub async fn shell_input(&self, session_id: &str, data: &str) {
        self.pty_manager.write_input(session_id, data.as_bytes());
    }

    pub fn shell_resize(&self, session_id: &str, cols: u16, rows: u16) {
        self.pty_manager.resize(session_id, cols, rows);
    }

    pub async fn handle_permission_response(&self, request_id: &str, approved: bool) {
        if let Some((_, tx)) = self.permission_channels.remove(request_id) {
            tx.send(approved).ok();
        }
    }

    pub async fn cleanup_all(&self) {
        let ids: Vec<String> = self.sessions.iter().map(|e| e.key().clone()).collect();
        for id in ids {
            self.abort_session(&id).await;
        }
        self.pty_manager.cleanup_all();
    }

    async fn send_response(&self, sender: &WsSender, response: &BrokerResponse) {
        if let Ok(json) = serde_json::to_string(response) {
            let mut s = sender.lock().await;
            let _ = s.send(Message::Text(json.into())).await;
        }
    }
}
