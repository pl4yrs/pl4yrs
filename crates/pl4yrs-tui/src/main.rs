use crate::app::App;
use clap::Parser;
use reqwest::Url;

pub mod app;
pub mod event;
pub mod pl4yrs_client;
pub mod ui;

const DEFAULT_PL4YRS_SERVER_URL: &str = "http://localhost:4243";

#[derive(Parser, Debug)]
#[command(version, about = "Pl4yrs TUI client")]
struct Args {
    /// Pl4yrs Server URL
    #[arg(short, long, env = "PL4YRS_SERVER_URL", default_value_t = Url::parse(DEFAULT_PL4YRS_SERVER_URL).unwrap())]
    pl4yrs_server_url: Url,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let args = Args::parse();
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app =
        App::new(args.pl4yrs_server_url).map_err(|error| color_eyre::eyre::eyre!("{error:#}"))?;
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}
