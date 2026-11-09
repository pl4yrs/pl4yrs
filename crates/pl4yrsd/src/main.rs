mod api;
mod player;

use crate::api::state;
use crate::player::engine;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

use tracing::{debug, info};

const DEFAULT_HTTP_PORT: u16 = 4242;

#[derive(Parser, Debug)]
#[command(name = "pl4yrsd", version, about = "pl4y.rs audio daemon")]
struct Args {
    /// TCP port for the HTTP API and web UI
    #[arg(short, long, env = "PL4YRSD_PORT", default_value_t = DEFAULT_HTTP_PORT)]
    port: u16,
    /// The file to play upon startup
    #[arg(long, env = "PL4YRSD_STARTUP_FILE")]
    file: Option<PathBuf>,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            // XXX: change to info before shipping
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pl4yrsd=debug,axum=info,rodio=info,tower_http=info".into()),
        )
        .init();

    let args = Args::parse();

    let player = engine::spawn();
    if let Some(path) = args.file {
        debug!(
            "daemon started with initial playback file {}",
            path.display()
        );
        player.send(player::PlayerCommand::Enqueue { track: path });
        player.send(player::PlayerCommand::Play);
    }

    let state = state::AppState::new(player);
    run_app(state, args.port).await?;
    Ok(())
}

async fn run_app(state: state::AppState, port: u16) -> Result<()> {
    let app = api::router::get_router(state);

    let listen_addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .with_context(|| format!("Failed to bind to port {}", port))?;
    info!(listen_addr, port, "http api listening");
    axum::serve(listener, app)
        .await
        .with_context(|| "Failed to run axum server")?;
    Ok(())
}
