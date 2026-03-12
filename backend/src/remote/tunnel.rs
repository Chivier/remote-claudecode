use dashmap::DashMap;
use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::db::servers::Server;
use crate::db::Database;

#[derive(Debug, Clone, PartialEq)]
pub enum TunnelStatus {
    Connected,
    Connecting,
    Disconnected,
    Error(String),
}

struct TunnelState {
    ssh_process: Option<Child>,
    local_port: u16,
    status: TunnelStatus,
    idle_timer: Option<JoinHandle<()>>,
    retry_count: u32,
    last_activity: Instant,
    idle_timeout_secs: u64,
}

pub struct TunnelManager {
    tunnels: DashMap<String, Arc<Mutex<TunnelState>>>,
    db: Database,
    port_range: Range<u16>,
    allocated_ports: DashMap<u16, String>, // port -> server_id
}

impl TunnelManager {
    pub fn new(db: Database) -> Self {
        Self {
            tunnels: DashMap::new(),
            db,
            port_range: 14000..15000,
            allocated_ports: DashMap::new(),
        }
    }

    /// Ensure a tunnel is open for the given server, return the local port
    pub async fn ensure_tunnel(&self, server_id: &str) -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
        // Check if tunnel already exists and is connected
        if let Some(entry) = self.tunnels.get(server_id) {
            let state = entry.lock().await;
            if state.status == TunnelStatus::Connected {
                return Ok(state.local_port);
            }
        }

        let server = self
            .db
            .get_server(server_id)?
            .ok_or_else(|| format!("Server {} not found", server_id))?;

        if server.is_local {
            return Err("Cannot create tunnel for local server".into());
        }

        let local_port = self.allocate_port(server_id)?;

        let state = Arc::new(Mutex::new(TunnelState {
            ssh_process: None,
            local_port,
            status: TunnelStatus::Connecting,
            idle_timer: None,
            retry_count: 0,
            last_activity: Instant::now(),
            idle_timeout_secs: server.idle_timeout_secs as u64,
        }));

        self.tunnels.insert(server_id.to_string(), state.clone());

        // Spawn SSH process
        match self.spawn_ssh(&server, local_port).await {
            Ok(child) => {
                let mut s = state.lock().await;
                s.ssh_process = Some(child);
                s.status = TunnelStatus::Connected;
                s.last_activity = Instant::now();

                // Update DB
                self.db
                    .update_server_tunnel_port(server_id, Some(local_port as i32))
                    .ok();
                self.db.update_server_connected(server_id).ok();

                tracing::info!(
                    "SSH tunnel established for server {} on port {}",
                    server_id,
                    local_port
                );

                // Start idle timer
                self.start_idle_timer(server_id, state.clone());

                Ok(local_port)
            }
            Err(e) => {
                let mut s = state.lock().await;
                s.status = TunnelStatus::Error(e.to_string());
                self.tunnels.remove(server_id);
                self.allocated_ports.remove(&local_port);
                Err(e)
            }
        }
    }

    /// Close a tunnel for the given server
    pub async fn close_tunnel(&self, server_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some((_, state)) = self.tunnels.remove(server_id) {
            let mut s = state.lock().await;
            if let Some(ref mut process) = s.ssh_process {
                process.kill().await.ok();
            }
            if let Some(timer) = s.idle_timer.take() {
                timer.abort();
            }
            self.allocated_ports.remove(&s.local_port);
            self.db.update_server_tunnel_port(server_id, None).ok();
            s.status = TunnelStatus::Disconnected;
            tracing::info!("SSH tunnel closed for server {}", server_id);
        }
        Ok(())
    }

    pub fn get_status(&self, server_id: &str) -> TunnelStatus {
        self.tunnels
            .get(server_id)
            .map(|_| {
                // Can't await in sync context, return based on existence
                TunnelStatus::Connected
            })
            .unwrap_or(TunnelStatus::Disconnected)
    }

    pub async fn get_status_async(&self, server_id: &str) -> TunnelStatus {
        match self.tunnels.get(server_id) {
            Some(entry) => {
                let state = entry.lock().await;
                state.status.clone()
            }
            None => TunnelStatus::Disconnected,
        }
    }

    pub fn reset_idle_timer(&self, server_id: &str) {
        if let Some(entry) = self.tunnels.get(server_id) {
            // We start a new idle timer in the background
            let state = entry.clone();
            let sid = server_id.to_string();
            self.start_idle_timer(&sid, state);
        }
    }

    /// Shutdown all tunnels
    pub async fn shutdown(&self) {
        let server_ids: Vec<String> = self.tunnels.iter().map(|e| e.key().clone()).collect();
        for sid in server_ids {
            self.close_tunnel(&sid).await.ok();
        }
    }

    fn allocate_port(&self, server_id: &str) -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
        // Check if server already has an allocated port
        for entry in self.allocated_ports.iter() {
            if entry.value() == server_id {
                return Ok(*entry.key());
            }
        }

        // Find an available port in range
        for port in self.port_range.clone() {
            if !self.allocated_ports.contains_key(&port) {
                self.allocated_ports.insert(port, server_id.to_string());
                return Ok(port);
            }
        }

        Err("No available ports in range 14000-14999".into())
    }

    async fn spawn_ssh(
        &self,
        server: &Server,
        local_port: u16,
    ) -> Result<Child, Box<dyn std::error::Error + Send + Sync>> {
        let mut cmd = Command::new("ssh");
        cmd.arg("-N") // No remote command
            .arg("-L")
            .arg(format!(
                "{}:127.0.0.1:{}",
                local_port, server.broker_port
            ))
            .arg("-o").arg("ServerAliveInterval=30")
            .arg("-o").arg("ServerAliveCountMax=3")
            .arg("-o").arg("ExitOnForwardFailure=yes")
            .arg("-o").arg("ConnectTimeout=10")
            .arg("-o").arg("StrictHostKeyChecking=accept-new")
            .arg("-p").arg(server.ssh_port.to_string());

        if let Some(ref key_path) = server.ssh_key_path {
            cmd.arg("-i").arg(key_path);
        }

        let target = if server.ssh_user.is_empty() {
            server.hostname.clone()
        } else {
            format!("{}@{}", server.ssh_user, server.hostname)
        };
        cmd.arg(&target);

        let child = cmd
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Give SSH a moment to establish the tunnel
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        Ok(child)
    }

    fn start_idle_timer(&self, server_id: &str, state: Arc<Mutex<TunnelState>>) {
        let sid = server_id.to_string();
        let tunnels = self.tunnels.clone();
        let allocated_ports = self.allocated_ports.clone();
        let db = self.db.clone();

        tokio::spawn(async move {
            let timeout = {
                let s = state.lock().await;
                // Cancel any existing timer
                if let Some(ref timer) = s.idle_timer {
                    timer.abort();
                }
                s.idle_timeout_secs
            };

            tokio::time::sleep(tokio::time::Duration::from_secs(timeout)).await;

            // Check if still idle
            let should_close = {
                let s = state.lock().await;
                s.last_activity.elapsed().as_secs() >= timeout
            };

            if should_close {
                tracing::info!("Idle timeout reached for server {}, closing tunnel", sid);
                if let Some((_, state)) = tunnels.remove(&sid) {
                    let mut s = state.lock().await;
                    if let Some(ref mut process) = s.ssh_process {
                        process.kill().await.ok();
                    }
                    allocated_ports.remove(&s.local_port);
                    db.update_server_tunnel_port(&sid, None).ok();
                    s.status = TunnelStatus::Disconnected;
                }
            }
        });
    }
}
