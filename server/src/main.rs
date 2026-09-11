use axum::Router;
use clap::Parser;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use hermesgate::{
    api::{self, state::AppState},
    config::Config,
    db, mcp, ui,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::parse();

    // In MCP STDIO mode, logs must go to stderr so stdout is reserved for JSON-RPC
    if config.mcp_stdio {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "hermesgate=warn".into()),
            )
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
            .init();

        let token = config.token.as_deref().unwrap_or_else(|| {
            eprintln!("Error: --token is required when running in --mcp-stdio mode");
            std::process::exit(1);
        });

        let pool = db::init_pool(&config.db_path)?;
        return mcp::stdio::run_stdio(pool, token).await;
    }

    // Normal server mode: logs go to stdout
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hermesgate=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🚀 Initializing HermesGate Server v{}", env!("CARGO_PKG_VERSION"));

    // 1. Initialize SQLite database pool
    let pool = db::init_pool(&config.db_path)?;
    info!("📦 Connected to SQLite database: {}", config.db_path);

    // 2. Ensure Admin Token exists
    let admin_token = db::ensure_admin_token(&pool, config.admin_token.as_deref())?;

    // 3. Setup broadcast channel for real-time WebSocket events
    let (tx, _) = broadcast::channel(500);

    let server_url = Arc::new(format!("http://{}:{}", if config.host == "0.0.0.0" { "localhost" } else { &config.host }, config.port));

    let state = AppState {
        pool: pool.clone(),
        tx,
        server_url: server_url.clone(),
    };

    // 4. Build application router
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest("/api/v1", api::router())
        .nest("/mcp", mcp::router())
        .fallback(ui::static_handler)
        .layer(cors)
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    info!("===========================================================");
    info!("🏛️  HermesGate Server is ready!");
    info!("🌐 Web UI Dashboard:  {}", *server_url);
    info!("🔑 Master Admin Token: {}", admin_token);
    info!("🤖 MCP SSE Endpoint:   {}/mcp/sse?token=<TOKEN>", *server_url);
    info!("📡 REST API Endpoint:  {}/api/v1/sms/forward", *server_url);
    info!("===========================================================");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
