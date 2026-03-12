use rand::Rng;
use serde::{Deserialize, Serialize};

use super::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKey {
    pub id: i64,
    pub key_name: String,
    pub api_key: String,
    pub created_at: Option<String>,
    pub last_used: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credential {
    pub id: i64,
    pub credential_name: String,
    pub credential_type: String,
    pub description: Option<String>,
    pub created_at: Option<String>,
    pub is_active: bool,
}

fn generate_api_key() -> String {
    let bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen::<u8>()).collect();
    format!("ck_{}", hex::encode(&bytes))
}

// We need hex encoding - use a simple inline version
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

impl Database {
    // --- API Keys ---

    pub fn create_api_key(&self, user_id: i64, key_name: &str) -> Result<ApiKey, rusqlite::Error> {
        let api_key = generate_api_key();
        let conn = self.conn();
        conn.execute(
            "INSERT INTO api_keys (user_id, key_name, api_key) VALUES (?1, ?2, ?3)",
            rusqlite::params![user_id, key_name, api_key],
        )?;
        let id = conn.last_insert_rowid();
        Ok(ApiKey {
            id,
            key_name: key_name.to_string(),
            api_key,
            created_at: None,
            last_used: None,
            is_active: true,
        })
    }

    pub fn get_api_keys(&self, user_id: i64) -> Result<Vec<ApiKey>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, key_name, api_key, created_at, last_used, is_active FROM api_keys WHERE user_id = ?1 ORDER BY created_at DESC",
        )?;
        let keys = stmt
            .query_map(rusqlite::params![user_id], |row| {
                Ok(ApiKey {
                    id: row.get(0)?,
                    key_name: row.get(1)?,
                    api_key: row.get(2)?,
                    created_at: row.get(3)?,
                    last_used: row.get(4)?,
                    is_active: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(keys)
    }

    pub fn validate_api_key(&self, api_key: &str) -> Result<Option<(i64, String)>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT u.id, u.username, ak.id as api_key_id
             FROM api_keys ak JOIN users u ON ak.user_id = u.id
             WHERE ak.api_key = ?1 AND ak.is_active = 1 AND u.is_active = 1",
        )?;
        match stmt.query_row(rusqlite::params![api_key], |row| {
            let user_id: i64 = row.get(0)?;
            let username: String = row.get(1)?;
            let ak_id: i64 = row.get(2)?;
            Ok((user_id, username, ak_id))
        }) {
            Ok((user_id, username, ak_id)) => {
                // Update last_used
                conn.execute(
                    "UPDATE api_keys SET last_used = CURRENT_TIMESTAMP WHERE id = ?1",
                    rusqlite::params![ak_id],
                )
                .ok();
                Ok(Some((user_id, username)))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn delete_api_key(&self, user_id: i64, key_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        let changes = conn.execute(
            "DELETE FROM api_keys WHERE id = ?1 AND user_id = ?2",
            rusqlite::params![key_id, user_id],
        )?;
        Ok(changes > 0)
    }

    pub fn toggle_api_key(
        &self,
        user_id: i64,
        key_id: i64,
        is_active: bool,
    ) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        let changes = conn.execute(
            "UPDATE api_keys SET is_active = ?1 WHERE id = ?2 AND user_id = ?3",
            rusqlite::params![is_active, key_id, user_id],
        )?;
        Ok(changes > 0)
    }

    // --- User Credentials ---

    pub fn create_credential(
        &self,
        user_id: i64,
        name: &str,
        cred_type: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<Credential, rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO user_credentials (user_id, credential_name, credential_type, credential_value, description) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![user_id, name, cred_type, value, description],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Credential {
            id,
            credential_name: name.to_string(),
            credential_type: cred_type.to_string(),
            description: description.map(|s| s.to_string()),
            created_at: None,
            is_active: true,
        })
    }

    pub fn get_credentials(
        &self,
        user_id: i64,
        cred_type: Option<&str>,
    ) -> Result<Vec<Credential>, rusqlite::Error> {
        let conn = self.conn();
        let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(ct) = cred_type {
            (
                "SELECT id, credential_name, credential_type, description, created_at, is_active FROM user_credentials WHERE user_id = ?1 AND credential_type = ?2 ORDER BY created_at DESC".to_string(),
                vec![Box::new(user_id), Box::new(ct.to_string())],
            )
        } else {
            (
                "SELECT id, credential_name, credential_type, description, created_at, is_active FROM user_credentials WHERE user_id = ?1 ORDER BY created_at DESC".to_string(),
                vec![Box::new(user_id)],
            )
        };
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let creds = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(Credential {
                    id: row.get(0)?,
                    credential_name: row.get(1)?,
                    credential_type: row.get(2)?,
                    description: row.get(3)?,
                    created_at: row.get(4)?,
                    is_active: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(creds)
    }

    pub fn get_active_credential(
        &self,
        user_id: i64,
        cred_type: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT credential_value FROM user_credentials WHERE user_id = ?1 AND credential_type = ?2 AND is_active = 1 ORDER BY created_at DESC LIMIT 1",
        )?;
        match stmt.query_row(rusqlite::params![user_id, cred_type], |row| {
            row.get::<_, String>(0)
        }) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn delete_credential(&self, user_id: i64, cred_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        let changes = conn.execute(
            "DELETE FROM user_credentials WHERE id = ?1 AND user_id = ?2",
            rusqlite::params![cred_id, user_id],
        )?;
        Ok(changes > 0)
    }

    pub fn toggle_credential(
        &self,
        user_id: i64,
        cred_id: i64,
        is_active: bool,
    ) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        let changes = conn.execute(
            "UPDATE user_credentials SET is_active = ?1 WHERE id = ?2 AND user_id = ?3",
            rusqlite::params![is_active, cred_id, user_id],
        )?;
        Ok(changes > 0)
    }

    // --- App Config ---

    pub fn get_config(&self, key: &str) -> Result<Option<String>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT value FROM app_config WHERE key = ?1")?;
        match stmt.query_row(rusqlite::params![key], |row| row.get::<_, String>(0)) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn set_config(&self, key: &str, value: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO app_config (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    pub fn get_or_create_jwt_secret(&self) -> String {
        if let Ok(Some(secret)) = self.get_config("jwt_secret") {
            return secret;
        }
        let secret: String = (0..64)
            .map(|_| format!("{:02x}", rand::thread_rng().gen::<u8>()))
            .collect();
        self.set_config("jwt_secret", &secret).ok();
        secret
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;

    fn setup_db() -> (Database, i64) {
        let db = Database::in_memory().unwrap();
        let user = db.create_user("testuser", "hash").unwrap();
        (db, user.id)
    }

    // --- API Key tests ---

    #[test]
    fn test_create_api_key() {
        let (db, uid) = setup_db();
        let key = db.create_api_key(uid, "my-key").unwrap();
        assert_eq!(key.key_name, "my-key");
        assert!(key.api_key.starts_with("ck_"));
        assert_eq!(key.api_key.len(), 3 + 64); // "ck_" + 64 hex chars
        assert!(key.is_active);
    }

    #[test]
    fn test_list_api_keys() {
        let (db, uid) = setup_db();
        db.create_api_key(uid, "key1").unwrap();
        db.create_api_key(uid, "key2").unwrap();
        let keys = db.get_api_keys(uid).unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_validate_api_key() {
        let (db, uid) = setup_db();
        let key = db.create_api_key(uid, "valid-key").unwrap();
        let result = db.validate_api_key(&key.api_key).unwrap();
        assert!(result.is_some());
        let (user_id, username) = result.unwrap();
        assert_eq!(user_id, uid);
        assert_eq!(username, "testuser");
    }

    #[test]
    fn test_validate_invalid_api_key() {
        let (db, _) = setup_db();
        let result = db.validate_api_key("ck_nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_api_key() {
        let (db, uid) = setup_db();
        let key = db.create_api_key(uid, "to-delete").unwrap();
        assert!(db.delete_api_key(uid, key.id).unwrap());
        assert_eq!(db.get_api_keys(uid).unwrap().len(), 0);
    }

    #[test]
    fn test_toggle_api_key() {
        let (db, uid) = setup_db();
        let key = db.create_api_key(uid, "toggleable").unwrap();
        // Disable
        db.toggle_api_key(uid, key.id, false).unwrap();
        // Should not validate when disabled
        let result = db.validate_api_key(&key.api_key).unwrap();
        assert!(result.is_none());
        // Re-enable
        db.toggle_api_key(uid, key.id, true).unwrap();
        let result = db.validate_api_key(&key.api_key).unwrap();
        assert!(result.is_some());
    }

    // --- Credential tests ---

    #[test]
    fn test_create_credential() {
        let (db, uid) = setup_db();
        let cred = db.create_credential(uid, "github-token", "github", "ghp_xxx", Some("My GH token")).unwrap();
        assert_eq!(cred.credential_name, "github-token");
        assert_eq!(cred.credential_type, "github");
    }

    #[test]
    fn test_get_credentials() {
        let (db, uid) = setup_db();
        db.create_credential(uid, "gh", "github", "ghp_1", None).unwrap();
        db.create_credential(uid, "gl", "gitlab", "gl_1", None).unwrap();

        let all = db.get_credentials(uid, None).unwrap();
        assert_eq!(all.len(), 2);

        let github_only = db.get_credentials(uid, Some("github")).unwrap();
        assert_eq!(github_only.len(), 1);
        assert_eq!(github_only[0].credential_type, "github");
    }

    #[test]
    fn test_get_active_credential() {
        let (db, uid) = setup_db();
        db.create_credential(uid, "gh", "github", "ghp_active", None).unwrap();
        let val = db.get_active_credential(uid, "github").unwrap();
        assert_eq!(val.unwrap(), "ghp_active");

        let none = db.get_active_credential(uid, "nonexistent").unwrap();
        assert!(none.is_none());
    }

    #[test]
    fn test_toggle_credential() {
        let (db, uid) = setup_db();
        let cred = db.create_credential(uid, "gh", "github", "token", None).unwrap();
        db.toggle_credential(uid, cred.id, false).unwrap();
        let val = db.get_active_credential(uid, "github").unwrap();
        assert!(val.is_none());
    }

    // --- App Config tests ---

    #[test]
    fn test_app_config() {
        let (db, _) = setup_db();
        assert!(db.get_config("nonexistent").unwrap().is_none());

        db.set_config("test_key", "test_value").unwrap();
        assert_eq!(db.get_config("test_key").unwrap().unwrap(), "test_value");

        // Upsert
        db.set_config("test_key", "updated").unwrap();
        assert_eq!(db.get_config("test_key").unwrap().unwrap(), "updated");
    }

    #[test]
    fn test_jwt_secret_generation() {
        let (db, _) = setup_db();
        let secret1 = db.get_or_create_jwt_secret();
        assert_eq!(secret1.len(), 128); // 64 bytes * 2 hex chars
        let secret2 = db.get_or_create_jwt_secret();
        assert_eq!(secret1, secret2); // Should return same secret
    }
}
