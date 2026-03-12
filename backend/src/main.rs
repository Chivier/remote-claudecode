mod api;
mod auth;
mod config;
mod db;
mod providers;
mod remote;
mod utils;
mod ws;

use axum::{
    extract::ws::WebSocketUpgrade,
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use auth::middleware::AppState;
use config::Config;
use db::Database;
use remote::dispatcher::ConnectionDispatcher;
use remote::tunnel::TunnelManager;

#[tokio::main]
async fn main() {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cloudcli_backend=info,tower_http=info".into()),
        )
        .init();

    // Load configuration
    let config = Config::from_env();
    tracing::info!("Starting CloudCLI Backend on {}:{}", config.host, config.port);

    // Initialize database
    let db = Database::new(&config).expect("Failed to initialize database");
    tracing::info!("Database initialized at {:?}", config.database_path);

    // Get or create JWT secret
    let jwt_secret = config
        .jwt_secret
        .clone()
        .unwrap_or_else(|| db.get_or_create_jwt_secret());

    // Initialize tunnel manager for remote servers
    let tunnel_manager = Arc::new(TunnelManager::new(db.clone()));

    // Create connection dispatcher
    let dispatcher = ConnectionDispatcher::new(
        tunnel_manager.clone(),
        config.broker_port,
        db.clone(),
    );

    // Create app state
    let app_state = AppState {
        db: db.clone(),
        jwt_secret,
        config: Arc::new(config.clone()),
    };

    // Build router
    let api_router = api::create_router();

    let dispatcher_chat = dispatcher.clone();
    let dispatcher_shell = dispatcher.clone();
    let state_chat = app_state.clone();
    let state_shell = app_state.clone();

    let app = Router::new()
        .merge(api_router)
        // WebSocket endpoints
        .route(
            "/ws",
            get(move |ws: WebSocketUpgrade| async move {
                ws.on_upgrade(move |socket| {
                    ws::chat::handle_chat_connection(socket, state_chat, dispatcher_chat)
                })
            }),
        )
        .route(
            "/shell",
            get(move |ws: WebSocketUpgrade| async move {
                ws.on_upgrade(move |socket| {
                    ws::shell::handle_shell_connection(socket, state_shell, dispatcher_shell)
                })
            }),
        )
        // Static files (frontend)
        .fallback_service(ServeDir::new(&config.frontend_dist))
        // Middleware
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    // Bind and serve
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("Invalid address");

    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    // Graceful shutdown
    let tunnel_manager_shutdown = tunnel_manager.clone();
    let shutdown_signal = async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C handler");
        tracing::info!("Shutting down...");
        tunnel_manager_shutdown.shutdown().await;
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .expect("Server error");
}
