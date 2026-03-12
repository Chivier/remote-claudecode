// PTY management module
// This will use portable-pty for local PTY operations
// For remote PTY, messages are forwarded to the broker

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

pub struct PtySession {
    pair: portable_pty::PtyPair,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    reader: Box<dyn Read + Send>,
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

    pub fn create_session(
        &self,
        session_id: &str,
        cols: u16,
        rows: u16,
        cwd: &str,
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
        let reader = pair.master.try_clone_reader()?;

        let session = PtySession {
            pair,
            child,
            writer,
            reader,
        };

        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session_id.to_string(), session);

        Ok(())
    }

    pub fn write_input(
        &self,
        session_id: &str,
        data: &[u8],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            session.writer.write_all(data)?;
            session.writer.flush()?;
        }
        Ok(())
    }

    pub fn resize(
        &self,
        session_id: &str,
        cols: u16,
        rows: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get(session_id) {
            session.pair.master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })?;
        }
        Ok(())
    }

    pub fn close_session(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(mut session) = sessions.remove(session_id) {
            session.child.kill().ok();
        }
    }
}
