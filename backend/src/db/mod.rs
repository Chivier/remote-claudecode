pub mod schema;
pub mod users;
pub mod servers;
pub mod credentials;
pub mod sessions;

use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use crate::config::Config;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(config: &Config) -> Result<Self, rusqlite::Error> {
        // Ensure parent directory exists
        if let Some(parent) = config.database_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&config.database_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.run_migrations()?;

        Ok(db)
    }

    /// Open an in-memory database (for testing)
    #[allow(dead_code)]
    pub fn in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("database lock poisoned")
    }

    fn run_migrations(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        schema::run_migrations(&conn)
    }
}
