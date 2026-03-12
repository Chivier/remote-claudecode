use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub host: String,
    pub database_path: PathBuf,
    pub jwt_secret: Option<String>,
    pub claude_cli_path: String,
    pub context_window: u64,
    pub is_platform: bool,
    pub frontend_dist: PathBuf,
    pub broker_port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3001),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            database_path: env::var("DATABASE_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("data/auth.db")),
            jwt_secret: env::var("JWT_SECRET").ok(),
            claude_cli_path: env::var("CLAUDE_CLI_PATH").unwrap_or_else(|_| "claude".to_string()),
            context_window: env::var("CONTEXT_WINDOW")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(160_000),
            is_platform: env::var("IS_PLATFORM")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            frontend_dist: env::var("FRONTEND_DIST")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("../frontend/dist")),
            broker_port: env::var("BROKER_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(19999),
        }
    }
}
