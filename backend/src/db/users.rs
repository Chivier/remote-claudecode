use serde::{Deserialize, Serialize};

use super::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: Option<String>,
    pub last_login: Option<String>,
    pub is_active: bool,
    pub git_name: Option<String>,
    pub git_email: Option<String>,
    pub has_completed_onboarding: bool,
}

/// Public-facing user info (no password hash)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub created_at: Option<String>,
    pub last_login: Option<String>,
}

impl Database {
    pub fn has_users(&self) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
        Ok(count > 0)
    }

    pub fn create_user(&self, username: &str, password_hash: &str) -> Result<UserInfo, rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO users (username, password_hash) VALUES (?1, ?2)",
            rusqlite::params![username, password_hash],
        )?;
        let id = conn.last_insert_rowid();
        Ok(UserInfo {
            id,
            username: username.to_string(),
            created_at: None,
            last_login: None,
        })
    }

    pub fn get_user_by_username(&self, username: &str) -> Result<Option<User>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, username, password_hash, created_at, last_login, is_active, git_name, git_email, has_completed_onboarding FROM users WHERE username = ?1 AND is_active = 1",
        )?;
        let user = stmt
            .query_row(rusqlite::params![username], |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    created_at: row.get(3)?,
                    last_login: row.get(4)?,
                    is_active: row.get(5)?,
                    git_name: row.get(6)?,
                    git_email: row.get(7)?,
                    has_completed_onboarding: row.get(8)?,
                })
            })
            .optional()?;
        Ok(user)
    }

    pub fn get_user_by_id(&self, user_id: i64) -> Result<Option<UserInfo>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, username, created_at, last_login FROM users WHERE id = ?1 AND is_active = 1",
        )?;
        let user = stmt
            .query_row(rusqlite::params![user_id], |row| {
                Ok(UserInfo {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    created_at: row.get(2)?,
                    last_login: row.get(3)?,
                })
            })
            .optional()?;
        Ok(user)
    }

    pub fn get_first_user(&self) -> Result<Option<UserInfo>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, username, created_at, last_login FROM users WHERE is_active = 1 LIMIT 1",
        )?;
        let user = stmt
            .query_row([], |row| {
                Ok(UserInfo {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    created_at: row.get(2)?,
                    last_login: row.get(3)?,
                })
            })
            .optional()?;
        Ok(user)
    }

    pub fn update_last_login(&self, user_id: i64) {
        let conn = self.conn();
        if let Err(e) = conn.execute(
            "UPDATE users SET last_login = CURRENT_TIMESTAMP WHERE id = ?1",
            rusqlite::params![user_id],
        ) {
            tracing::warn!("Failed to update last login: {}", e);
        }
    }

    pub fn update_git_config(
        &self,
        user_id: i64,
        git_name: &str,
        git_email: &str,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "UPDATE users SET git_name = ?1, git_email = ?2 WHERE id = ?3",
            rusqlite::params![git_name, git_email, user_id],
        )?;
        Ok(())
    }

    pub fn get_git_config(&self, user_id: i64) -> Result<(Option<String>, Option<String>), rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT git_name, git_email FROM users WHERE id = ?1")?;
        stmt.query_row(rusqlite::params![user_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
    }

    pub fn complete_onboarding(&self, user_id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "UPDATE users SET has_completed_onboarding = 1 WHERE id = ?1",
            rusqlite::params![user_id],
        )?;
        Ok(())
    }

    pub fn has_completed_onboarding(&self, user_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt =
            conn.prepare("SELECT has_completed_onboarding FROM users WHERE id = ?1")?;
        let val: bool = stmt.query_row(rusqlite::params![user_id], |row| row.get(0))?;
        Ok(val)
    }
}

// Extension trait for optional query results
trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;

    fn setup_db() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn test_has_users_empty() {
        let db = setup_db();
        assert!(!db.has_users().unwrap());
    }

    #[test]
    fn test_create_and_find_user() {
        let db = setup_db();
        let user = db.create_user("alice", "hashed_pw").unwrap();
        assert_eq!(user.username, "alice");
        assert!(user.id > 0);

        assert!(db.has_users().unwrap());
    }

    #[test]
    fn test_get_user_by_username() {
        let db = setup_db();
        db.create_user("bob", "hashed").unwrap();
        let user = db.get_user_by_username("bob").unwrap().unwrap();
        assert_eq!(user.username, "bob");
        assert_eq!(user.password_hash, "hashed");
        assert!(!user.has_completed_onboarding);
    }

    #[test]
    fn test_get_user_by_username_not_found() {
        let db = setup_db();
        let result = db.get_user_by_username("nobody").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_user_by_id() {
        let db = setup_db();
        let created = db.create_user("charlie", "pw").unwrap();
        let user = db.get_user_by_id(created.id).unwrap().unwrap();
        assert_eq!(user.username, "charlie");
    }

    #[test]
    fn test_get_first_user() {
        let db = setup_db();
        assert!(db.get_first_user().unwrap().is_none());
        db.create_user("first", "pw").unwrap();
        db.create_user("second", "pw").unwrap();
        let first = db.get_first_user().unwrap().unwrap();
        assert_eq!(first.username, "first");
    }

    #[test]
    fn test_duplicate_username() {
        let db = setup_db();
        db.create_user("alice", "pw1").unwrap();
        let result = db.create_user("alice", "pw2");
        assert!(result.is_err());
    }

    #[test]
    fn test_update_git_config() {
        let db = setup_db();
        let user = db.create_user("dev", "pw").unwrap();
        db.update_git_config(user.id, "Dev Name", "dev@example.com").unwrap();
        let (name, email) = db.get_git_config(user.id).unwrap();
        assert_eq!(name.unwrap(), "Dev Name");
        assert_eq!(email.unwrap(), "dev@example.com");
    }

    #[test]
    fn test_onboarding() {
        let db = setup_db();
        let user = db.create_user("new_user", "pw").unwrap();
        assert!(!db.has_completed_onboarding(user.id).unwrap());
        db.complete_onboarding(user.id).unwrap();
        assert!(db.has_completed_onboarding(user.id).unwrap());
    }

    #[test]
    fn test_update_last_login() {
        let db = setup_db();
        let user = db.create_user("login_user", "pw").unwrap();
        // Should not panic
        db.update_last_login(user.id);
        let fetched = db.get_user_by_username("login_user").unwrap().unwrap();
        assert!(fetched.last_login.is_some());
    }
}
