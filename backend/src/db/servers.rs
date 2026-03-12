use serde::{Deserialize, Serialize};

use super::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub id: String,
    pub user_id: i64,
    pub name: String,
    pub is_local: bool,
    pub hostname: String,
    pub ssh_port: i32,
    pub ssh_user: String,
    pub ssh_key_path: Option<String>,
    pub auth_method: String,
    pub broker_port: i32,
    pub default_work_dir: Option<String>,
    pub tunnel_local_port: Option<i32>,
    pub auto_update: bool,
    pub idle_timeout_secs: i32,
    pub is_active: bool,
    pub broker_version: Option<String>,
    pub last_connected_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateServerRequest {
    pub name: String,
    pub hostname: Option<String>,
    pub ssh_port: Option<i32>,
    pub ssh_user: Option<String>,
    pub ssh_key_path: Option<String>,
    pub auth_method: Option<String>,
    pub broker_port: Option<i32>,
    pub default_work_dir: Option<String>,
    pub auto_update: Option<bool>,
    pub idle_timeout_secs: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateServerRequest {
    pub name: Option<String>,
    pub hostname: Option<String>,
    pub ssh_port: Option<i32>,
    pub ssh_user: Option<String>,
    pub ssh_key_path: Option<String>,
    pub auth_method: Option<String>,
    pub broker_port: Option<i32>,
    pub default_work_dir: Option<String>,
    pub auto_update: Option<bool>,
    pub idle_timeout_secs: Option<i32>,
    pub is_active: Option<bool>,
}

impl Database {
    pub fn create_server(
        &self,
        user_id: i64,
        req: &CreateServerRequest,
    ) -> Result<Server, rusqlite::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        {
            let conn = self.conn();
            conn.execute(
                "INSERT INTO servers (id, user_id, name, hostname, ssh_port, ssh_user, ssh_key_path, auth_method, broker_port, default_work_dir, auto_update, idle_timeout_secs)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                rusqlite::params![
                    id,
                    user_id,
                    req.name,
                    req.hostname.as_deref().unwrap_or("127.0.0.1"),
                    req.ssh_port.unwrap_or(22),
                    req.ssh_user.as_deref().unwrap_or(""),
                    req.ssh_key_path,
                    req.auth_method.as_deref().unwrap_or("key"),
                    req.broker_port.unwrap_or(9999),
                    req.default_work_dir,
                    req.auto_update.unwrap_or(true),
                    req.idle_timeout_secs.unwrap_or(300),
                ],
            )?;
        } // Drop conn before calling get_server
        self.get_server(&id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn ensure_local_server(&self, user_id: i64) -> Result<Server, rusqlite::Error> {
        // Check if local server already exists
        if let Some(server) = self.get_server("local")? {
            return Ok(server);
        }
        {
            let conn = self.conn();
            conn.execute(
                "INSERT OR IGNORE INTO servers (id, user_id, name, is_local, auth_method, hostname, broker_port)
                 VALUES ('local', ?1, 'Local', 1, 'local', '127.0.0.1', 19999)",
                rusqlite::params![user_id],
            )?;
        } // Drop conn before calling get_server
        self.get_server("local")?.ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn get_server(&self, server_id: &str) -> Result<Option<Server>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, is_local, hostname, ssh_port, ssh_user, ssh_key_path, auth_method, broker_port, default_work_dir, tunnel_local_port, auto_update, idle_timeout_secs, is_active, broker_version, last_connected_at, created_at, updated_at FROM servers WHERE id = ?1",
        )?;
        let server = stmt
            .query_row(rusqlite::params![server_id], |row| {
                Ok(Server {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    name: row.get(2)?,
                    is_local: row.get(3)?,
                    hostname: row.get(4)?,
                    ssh_port: row.get(5)?,
                    ssh_user: row.get(6)?,
                    ssh_key_path: row.get(7)?,
                    auth_method: row.get(8)?,
                    broker_port: row.get(9)?,
                    default_work_dir: row.get(10)?,
                    tunnel_local_port: row.get(11)?,
                    auto_update: row.get(12)?,
                    idle_timeout_secs: row.get(13)?,
                    is_active: row.get(14)?,
                    broker_version: row.get(15)?,
                    last_connected_at: row.get(16)?,
                    created_at: row.get(17)?,
                    updated_at: row.get(18)?,
                })
            });
        match server {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn list_servers(&self, user_id: i64) -> Result<Vec<Server>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, user_id, name, is_local, hostname, ssh_port, ssh_user, ssh_key_path, auth_method, broker_port, default_work_dir, tunnel_local_port, auto_update, idle_timeout_secs, is_active, broker_version, last_connected_at, created_at, updated_at FROM servers WHERE user_id = ?1 ORDER BY is_local DESC, name ASC",
        )?;
        let servers = stmt
            .query_map(rusqlite::params![user_id], |row| {
                Ok(Server {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    name: row.get(2)?,
                    is_local: row.get(3)?,
                    hostname: row.get(4)?,
                    ssh_port: row.get(5)?,
                    ssh_user: row.get(6)?,
                    ssh_key_path: row.get(7)?,
                    auth_method: row.get(8)?,
                    broker_port: row.get(9)?,
                    default_work_dir: row.get(10)?,
                    tunnel_local_port: row.get(11)?,
                    auto_update: row.get(12)?,
                    idle_timeout_secs: row.get(13)?,
                    is_active: row.get(14)?,
                    broker_version: row.get(15)?,
                    last_connected_at: row.get(16)?,
                    created_at: row.get(17)?,
                    updated_at: row.get(18)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(servers)
    }

    pub fn update_server(
        &self,
        server_id: &str,
        user_id: i64,
        req: &UpdateServerRequest,
    ) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        // Build dynamic update query
        let mut sets = vec!["updated_at = datetime('now')".to_string()];
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        macro_rules! maybe_set {
            ($field:ident, $col:expr) => {
                if let Some(ref val) = req.$field {
                    sets.push(format!("{} = ?{}", $col, params.len() + 1));
                    params.push(Box::new(val.clone()));
                }
            };
        }

        maybe_set!(name, "name");
        maybe_set!(hostname, "hostname");
        maybe_set!(ssh_port, "ssh_port");
        maybe_set!(ssh_user, "ssh_user");
        maybe_set!(ssh_key_path, "ssh_key_path");
        maybe_set!(auth_method, "auth_method");
        maybe_set!(broker_port, "broker_port");
        maybe_set!(default_work_dir, "default_work_dir");
        maybe_set!(auto_update, "auto_update");
        maybe_set!(idle_timeout_secs, "idle_timeout_secs");
        maybe_set!(is_active, "is_active");

        let idx_server = params.len() + 1;
        let idx_user = params.len() + 2;
        params.push(Box::new(server_id.to_string()));
        params.push(Box::new(user_id));

        let sql = format!(
            "UPDATE servers SET {} WHERE id = ?{} AND user_id = ?{}",
            sets.join(", "),
            idx_server,
            idx_user
        );

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let changes = conn.execute(&sql, param_refs.as_slice())?;
        Ok(changes > 0)
    }

    pub fn delete_server(&self, server_id: &str, user_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        // Don't allow deleting the local server
        let changes = conn.execute(
            "DELETE FROM servers WHERE id = ?1 AND user_id = ?2 AND is_local = 0",
            rusqlite::params![server_id, user_id],
        )?;
        Ok(changes > 0)
    }

    pub fn update_server_tunnel_port(
        &self,
        server_id: &str,
        local_port: Option<i32>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "UPDATE servers SET tunnel_local_port = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![local_port, server_id],
        )?;
        Ok(())
    }

    pub fn update_server_connected(&self, server_id: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "UPDATE servers SET last_connected_at = datetime('now'), updated_at = datetime('now') WHERE id = ?1",
            rusqlite::params![server_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn setup_db() -> (Database, i64) {
        let db = Database::in_memory().unwrap();
        let user = db.create_user("testuser", "hash").unwrap();
        (db, user.id)
    }

    #[test]
    fn test_ensure_local_server() {
        let (db, uid) = setup_db();
        let server = db.ensure_local_server(uid).unwrap();
        assert_eq!(server.id, "local");
        assert!(server.is_local);
        assert_eq!(server.name, "Local");
        assert_eq!(server.hostname, "127.0.0.1");
        assert_eq!(server.broker_port, 19999);

        // Calling again should return same server
        let server2 = db.ensure_local_server(uid).unwrap();
        assert_eq!(server2.id, "local");
    }

    #[test]
    fn test_create_remote_server() {
        let (db, uid) = setup_db();
        let req = CreateServerRequest {
            name: "GPU Server".to_string(),
            hostname: Some("192.168.1.100".to_string()),
            ssh_port: Some(22),
            ssh_user: Some("admin".to_string()),
            ssh_key_path: Some("/home/user/.ssh/id_rsa".to_string()),
            auth_method: Some("key".to_string()),
            broker_port: Some(9999),
            default_work_dir: Some("/home/admin/work".to_string()),
            auto_update: Some(true),
            idle_timeout_secs: Some(600),
        };
        let server = db.create_server(uid, &req).unwrap();
        assert_eq!(server.name, "GPU Server");
        assert_eq!(server.hostname, "192.168.1.100");
        assert!(!server.is_local);
        assert_eq!(server.ssh_user, "admin");
        assert_eq!(server.idle_timeout_secs, 600);
    }

    #[test]
    fn test_list_servers() {
        let (db, uid) = setup_db();
        db.ensure_local_server(uid).unwrap();
        let req = CreateServerRequest {
            name: "Remote".to_string(),
            hostname: Some("10.0.0.1".to_string()),
            ssh_port: None,
            ssh_user: None,
            ssh_key_path: None,
            auth_method: None,
            broker_port: None,
            default_work_dir: None,
            auto_update: None,
            idle_timeout_secs: None,
        };
        db.create_server(uid, &req).unwrap();

        let servers = db.list_servers(uid).unwrap();
        assert_eq!(servers.len(), 2);
        // Local should come first (ORDER BY is_local DESC)
        assert!(servers[0].is_local);
    }

    #[test]
    fn test_update_server() {
        let (db, uid) = setup_db();
        let req = CreateServerRequest {
            name: "Old Name".to_string(),
            hostname: Some("10.0.0.1".to_string()),
            ssh_port: None,
            ssh_user: None,
            ssh_key_path: None,
            auth_method: None,
            broker_port: None,
            default_work_dir: None,
            auto_update: None,
            idle_timeout_secs: None,
        };
        let server = db.create_server(uid, &req).unwrap();

        let update = UpdateServerRequest {
            name: Some("New Name".to_string()),
            hostname: Some("10.0.0.2".to_string()),
            ssh_port: None,
            ssh_user: None,
            ssh_key_path: None,
            auth_method: None,
            broker_port: None,
            default_work_dir: None,
            auto_update: None,
            idle_timeout_secs: None,
            is_active: None,
        };
        assert!(db.update_server(&server.id, uid, &update).unwrap());

        let updated = db.get_server(&server.id).unwrap().unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.hostname, "10.0.0.2");
    }

    #[test]
    fn test_delete_server() {
        let (db, uid) = setup_db();
        db.ensure_local_server(uid).unwrap();
        let req = CreateServerRequest {
            name: "Deletable".to_string(),
            hostname: None,
            ssh_port: None,
            ssh_user: None,
            ssh_key_path: None,
            auth_method: None,
            broker_port: None,
            default_work_dir: None,
            auto_update: None,
            idle_timeout_secs: None,
        };
        let server = db.create_server(uid, &req).unwrap();
        assert!(db.delete_server(&server.id, uid).unwrap());
        assert!(db.get_server(&server.id).unwrap().is_none());

        // Cannot delete local server
        assert!(!db.delete_server("local", uid).unwrap());
    }

    #[test]
    fn test_update_tunnel_port() {
        let (db, uid) = setup_db();
        let req = CreateServerRequest {
            name: "Tunnel Test".to_string(),
            hostname: None,
            ssh_port: None,
            ssh_user: None,
            ssh_key_path: None,
            auth_method: None,
            broker_port: None,
            default_work_dir: None,
            auto_update: None,
            idle_timeout_secs: None,
        };
        let server = db.create_server(uid, &req).unwrap();
        assert!(server.tunnel_local_port.is_none());

        db.update_server_tunnel_port(&server.id, Some(14001)).unwrap();
        let updated = db.get_server(&server.id).unwrap().unwrap();
        assert_eq!(updated.tunnel_local_port, Some(14001));
    }
}
