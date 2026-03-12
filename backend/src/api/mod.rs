pub mod auth;
pub mod commands;
pub mod git;
pub mod projects;
pub mod remote_servers;
pub mod settings;
pub mod user;

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::auth::middleware::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        // Health check
        .route("/api/health", get(health_check))
        // Auth
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/check", get(auth::check_auth))
        .route("/api/auth/has-users", get(auth::has_users))
        // Projects
        .route("/api/projects", get(projects::list_projects))
        .route("/api/projects/create-workspace", post(projects::create_workspace))
        // Settings - API Keys
        .route("/api/settings/api-keys", get(settings::get_api_keys))
        .route("/api/settings/api-keys", post(settings::create_api_key))
        .route("/api/settings/api-keys/{id}", delete(settings::delete_api_key))
        .route("/api/settings/api-keys/{id}/toggle", put(settings::toggle_api_key))
        // Settings - Credentials
        .route("/api/settings/credentials", get(settings::get_credentials))
        .route("/api/settings/credentials", post(settings::create_credential))
        .route("/api/settings/credentials/{id}", delete(settings::delete_credential))
        .route("/api/settings/credentials/{id}/toggle", put(settings::toggle_credential))
        // User
        .route("/api/user/git-config", get(user::get_git_config))
        .route("/api/user/git-config", post(user::update_git_config))
        .route("/api/user/onboarding", get(user::get_onboarding_status))
        .route("/api/user/onboarding/complete", post(user::complete_onboarding))
        // Git
        .route("/api/git/status", post(git::git_status))
        .route("/api/git/log", post(git::git_log))
        .route("/api/git/diff", post(git::git_diff))
        .route("/api/git/branch", post(git::git_branch))
        .route("/api/git/checkout", post(git::git_checkout))
        .route("/api/git/commit", post(git::git_commit))
        .route("/api/git/push", post(git::git_push))
        .route("/api/git/pull", post(git::git_pull))
        .route("/api/git/stash", post(git::git_stash))
        // Remote Servers
        .route("/api/remote-servers", get(remote_servers::list_servers))
        .route("/api/remote-servers", post(remote_servers::create_server))
        .route("/api/remote-servers/{id}", get(remote_servers::get_server))
        .route("/api/remote-servers/{id}", put(remote_servers::update_server))
        .route("/api/remote-servers/{id}", delete(remote_servers::delete_server))
        .route("/api/remote-servers/{id}/test", post(remote_servers::test_connection))
        .route("/api/remote-servers/{id}/deploy", post(remote_servers::deploy_broker))
        .route("/api/remote-servers/{id}/status", get(remote_servers::get_status))
        // Session names
        .route("/api/sessions/{provider}/{sessionId}/name", post(settings::set_session_name))
        .route("/api/sessions/{provider}/{sessionId}/name", get(settings::get_session_name))
        .route("/api/sessions/{provider}/{sessionId}/name", delete(settings::delete_session_name))
}

async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
