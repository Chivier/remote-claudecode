use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionName {
    pub session_id: String,
    pub provider: String,
    pub custom_name: String,
}

impl Database {
    pub fn set_session_name(
        &self,
        session_id: &str,
        provider: &str,
        custom_name: &str,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO session_names (session_id, provider, custom_name) VALUES (?1, ?2, ?3)
             ON CONFLICT(session_id, provider) DO UPDATE SET custom_name = excluded.custom_name, updated_at = CURRENT_TIMESTAMP",
            rusqlite::params![session_id, provider, custom_name],
        )?;
        Ok(())
    }

    pub fn get_session_name(
        &self,
        session_id: &str,
        provider: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT custom_name FROM session_names WHERE session_id = ?1 AND provider = ?2",
        )?;
        match stmt.query_row(rusqlite::params![session_id, provider], |row| {
            row.get::<_, String>(0)
        }) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn get_session_names(
        &self,
        session_ids: &[String],
        provider: &str,
    ) -> Result<HashMap<String, String>, rusqlite::Error> {
        if session_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let conn = self.conn();
        let placeholders: Vec<String> = (0..session_ids.len())
            .map(|i| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT session_id, custom_name FROM session_names WHERE session_id IN ({}) AND provider = ?{}",
            placeholders.join(","),
            session_ids.len() + 1
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> =
            session_ids.iter().map(|s| Box::new(s.clone()) as Box<dyn rusqlite::types::ToSql>).collect();
        params.push(Box::new(provider.to_string()));
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let mut map = HashMap::new();
        let mut rows = stmt.query(param_refs.as_slice())?;
        while let Some(row) = rows.next()? {
            let sid: String = row.get(0)?;
            let name: String = row.get(1)?;
            map.insert(sid, name);
        }
        Ok(map)
    }

    pub fn delete_session_name(
        &self,
        session_id: &str,
        provider: &str,
    ) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        let changes = conn.execute(
            "DELETE FROM session_names WHERE session_id = ?1 AND provider = ?2",
            rusqlite::params![session_id, provider],
        )?;
        Ok(changes > 0)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;

    fn setup_db() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn test_set_and_get_session_name() {
        let db = setup_db();
        db.set_session_name("sess-1", "claude", "My Chat").unwrap();
        let name = db.get_session_name("sess-1", "claude").unwrap();
        assert_eq!(name.unwrap(), "My Chat");
    }

    #[test]
    fn test_session_name_upsert() {
        let db = setup_db();
        db.set_session_name("sess-1", "claude", "Old Name").unwrap();
        db.set_session_name("sess-1", "claude", "New Name").unwrap();
        let name = db.get_session_name("sess-1", "claude").unwrap();
        assert_eq!(name.unwrap(), "New Name");
    }

    #[test]
    fn test_get_session_name_not_found() {
        let db = setup_db();
        let name = db.get_session_name("nonexistent", "claude").unwrap();
        assert!(name.is_none());
    }

    #[test]
    fn test_get_session_names_batch() {
        let db = setup_db();
        db.set_session_name("s1", "claude", "Chat 1").unwrap();
        db.set_session_name("s2", "claude", "Chat 2").unwrap();
        db.set_session_name("s3", "cursor", "Cursor Chat").unwrap();

        let ids = vec!["s1".to_string(), "s2".to_string(), "s3".to_string()];
        let names = db.get_session_names(&ids, "claude").unwrap();
        assert_eq!(names.len(), 2);
        assert_eq!(names["s1"], "Chat 1");
        assert_eq!(names["s2"], "Chat 2");
    }

    #[test]
    fn test_get_session_names_empty() {
        let db = setup_db();
        let names = db.get_session_names(&[], "claude").unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn test_delete_session_name() {
        let db = setup_db();
        db.set_session_name("sess-1", "claude", "Chat").unwrap();
        assert!(db.delete_session_name("sess-1", "claude").unwrap());
        assert!(db.get_session_name("sess-1", "claude").unwrap().is_none());

        // Deleting non-existent returns false
        assert!(!db.delete_session_name("nonexistent", "claude").unwrap());
    }
}
