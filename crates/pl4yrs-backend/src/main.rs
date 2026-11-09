mod api;
mod services;

use crate::api::state;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tracing::{debug, info};
use tracing_subscriber::prelude::*;

const DEFAULT_HTTP_PORT: u16 = 4243;
const DEFAULT_SENDSPIN_CONFIG_FILE: &str = "/etc/sendspin-cli.conf";
const DEFAULT_SENDSPIN_SOCK: &str = "/var/run/sendspin-cli/control.sock";

#[derive(Parser, Debug)]
#[command(
    name = "pl4yrs-backend",
    version,
    about = "pl4y.rs backend and API service"
)]
struct Args {
    /// TCP port for the HTTP API and web UI
    #[arg(short, long, env = "PL4YRS_BACKEND_PORT", default_value_t = DEFAULT_HTTP_PORT)]
    port: u16,
    /// Sendspin CLI Config File
    #[arg(long, env = "SENDSPIN_CONFIG_FILE", default_value = DEFAULT_SENDSPIN_CONFIG_FILE)]
    sendspin_config_file: Option<PathBuf>,
    /// Sendspin CLI control sock
    #[arg(long, env = "SENDSPIN_SOCK", default_value = DEFAULT_SENDSPIN_SOCK)]
    sendspin_sock: Option<PathBuf>,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer().with_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "pl4yrs_backend=debug,axum=info,tower_http=info".into()),
            ),
        )
        .init();

    debug!("Parsing cmdline arguments");
    let args = Args::parse();

    let state = state::AppState::new(
        args.sendspin_sock.unwrap(),
        args.sendspin_config_file.unwrap(),
    );
    state.sendspin_service.start_polling();
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
