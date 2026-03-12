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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_database_creation() {
        let db = Database::in_memory().expect("should create in-memory db");
        // Verify migrations ran by checking tables exist
        let conn = db.conn();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='users'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_database_has_all_tables() {
        let db = Database::in_memory().unwrap();
        let conn = db.conn();
        let tables = [
            "users",
            "api_keys",
            "user_credentials",
            "session_names",
            "app_config",
            "servers",
        ];
        for table in &tables {
            let count: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'",
                        table
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "table '{}' should exist", table);
        }
    }
}
