use futures::SinkExt;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio_tungstenite::tungstenite::Message;

use crate::protocol::BrokerResponse;

type WsSender = Arc<
    tokio::sync::Mutex<
        futures::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
            Message,
        >,
    >,
>;

struct PtySession {
    master: Box<dyn portable_pty::MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    _reader_task: Option<tokio::task::JoinHandle<()>>,
}

pub struct PtyManager {
    sessions: Arc<Mutex<HashMap<String, PtySession>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn create_session(
        &self,
        session_id: &str,
        cols: u16,
        rows: u16,
        cwd: &str,
        sender: WsSender,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new_default_prog();
        cmd.cwd(cwd);

        let child = pair.slave.spawn_command(cmd)?;
        let writer = pair.master.take_writer()?;
        let mut reader = pair.master.try_clone_reader()?;

        // Spawn reader task to forward PTY output to WebSocket
        let sid = session_id.to_string();
        let reader_task = tokio::task::spawn_blocking(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).to_string();
                        let response = BrokerResponse::ShellOutput {
                            session_id: sid.clone(),
                            data,
                        };
                        if let Ok(json) = serde_json::to_string(&response) {
                            let sender = sender.clone();
                            // Use a runtime handle to send from blocking context
                            let _ = tokio::runtime::Handle::current().block_on(async {
                                let mut s = sender.lock().await;
                                s.send(Message::Text(json.into())).await
                            });
                        }
                    }
                    Err(_) => break,
                }
            }

            // Send shell exit
            let response = BrokerResponse::ShellExit {
                session_id: sid,
                exit_code: 0,
            };
            if let Ok(json) = serde_json::to_string(&response) {
                let _ = tokio::runtime::Handle::current().block_on(async {
                    let mut s = sender.lock().await;
                    s.send(Message::Text(json.into())).await
                });
            }
        });

        let session = PtySession {
            master: pair.master,
            child,
            writer,
            _reader_task: Some(reader_task),
        };

        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session_id.to_string(), session);

        Ok(())
    }

    pub fn write_input(&self, session_id: &str, data: &[u8]) {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            session.writer.write_all(data).ok();
            session.writer.flush().ok();
        }
    }

    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) {
        let sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get(session_id) {
            session
                .master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .ok();
        }
    }

    pub fn cleanup_all(&self) {
        let mut sessions = self.sessions.lock().unwrap();
        for (_, mut session) in sessions.drain() {
            session.child.kill().ok();
        }
    }
}
