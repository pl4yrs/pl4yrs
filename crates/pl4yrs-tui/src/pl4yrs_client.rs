use crate::event::Event;
use anyhow::Result;
use pl4yrs_api::playback::PlayerStatus;
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, UnboundedSender};

pub enum BackendCommand {
    FetchStatus,
}

#[derive(Clone, Debug)]
pub enum BackendEvent {
    Connecting,
    Connected,
    StatusReceived(PlayerStatus),
    Disconnected(String),
    RequestFailed(String),
}

#[derive(Debug)]
pub struct Pl4yrsClient {
    command_tx: mpsc::Sender<BackendCommand>,
    command_task: tokio::task::JoinHandle<()>,
    sse_task: tokio::task::JoinHandle<()>,
}

impl Pl4yrsClient {
    pub fn start(base_url: reqwest::Url, event_tx: UnboundedSender<Event>) -> Result<Self> {
        let command_http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .build()?;
        let (command_tx, command_rx) = mpsc::channel(16);

        let command_task = tokio::spawn(run_command_worker(
            command_http,
            base_url.clone(),
            command_rx,
            event_tx.clone(),
        ));

        let sse_http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .build()?;

        let sse_task = tokio::spawn(run_sse_worker(sse_http, base_url, event_tx));

        Ok(Pl4yrsClient {
            command_tx,
            command_task,
            sse_task,
        })
    }

    pub fn try_send(
        &self,
        command: BackendCommand,
    ) -> Result<(), mpsc::error::TrySendError<BackendCommand>> {
        self.command_tx.try_send(command)
    }
}

async fn run_command_worker(
    http: reqwest::Client,
    base_url: reqwest::Url,
    mut commands: Receiver<BackendCommand>,
    event_tx: UnboundedSender<Event>,
) {
    while let Some(command) = commands.recv().await {
        match command {
            BackendCommand::FetchStatus => {
                let event = match fetch_status(&http, &base_url).await {
                    Ok(status) => BackendEvent::StatusReceived(status),
                    Err(error) => BackendEvent::RequestFailed(error.to_string()),
                };

                if event_tx.send(Event::Backend(event)).is_err() {
                    break;
                }
            }
        }
    }
}

async fn run_sse_worker(
    _http: reqwest::Client,
    _base_url: reqwest::Url,
    _event_tx: UnboundedSender<Event>,
) {
    ()
}

pub async fn fetch_status(http: &reqwest::Client, base_url: &reqwest::Url) -> Result<PlayerStatus> {
    let url = base_url.join("/status")?;

    Ok(http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<PlayerStatus>()
        .await?)
}

impl Drop for Pl4yrsClient {
    fn drop(&mut self) {
        self.command_task.abort();
        self.sse_task.abort();
    }
}
