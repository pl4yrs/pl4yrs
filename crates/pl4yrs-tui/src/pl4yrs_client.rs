use crate::event::Event;
use anyhow::Result;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use pl4yrs_api::control::{SetVolume, Volume};
use pl4yrs_api::playback::PlayerStatus;
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, UnboundedSender};

pub enum BackendCommand {
    FetchStatus,
    SetVolume(Volume),
}

#[derive(Clone, Debug)]
pub enum BackendEvent {
    Connecting,
    Connected,
    StatusReceived(PlayerStatus),
    Disconnected(String),
    RequestFailed(String),
    MalformedStatus(String),
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
            BackendCommand::SetVolume(volume) => {
                if let Err(error) = set_volume(&http, &base_url, volume).await {
                    let event = BackendEvent::RequestFailed(error.to_string());
                    if event_tx.send(Event::Backend(event)).is_err() {
                        break;
                    }
                };
            }
        }
    }
}

async fn run_sse_worker(
    http: reqwest::Client,
    base_url: reqwest::Url,
    event_tx: UnboundedSender<Event>,
) {
    let url = match base_url.join("/status/events") {
        Ok(url) => url,
        Err(error) => {
            let _ = event_tx.send(Event::Backend(BackendEvent::Disconnected(
                error.to_string(),
            )));
            return;
        }
    };

    loop {
        if event_tx
            .send(Event::Backend(BackendEvent::Connecting))
            .is_err()
        {
            return;
        }

        let result = tokio::select! {
            _ = event_tx.closed() => return,
            result = receive_status_events(&http, &url, &event_tx) => result,
        };

        let message = match result {
            Ok(()) => "Status stream closed".to_owned(),
            Err(error) => error.to_string(),
        };

        if event_tx
            .send(Event::Backend(BackendEvent::Disconnected(message)))
            .is_err()
        {
            return;
        }

        tokio::select! {
            _ = event_tx.closed() => return,
            _ = tokio::time::sleep(Duration::from_secs(2)) => {}
        }
    }
}

async fn receive_status_events(
    http: &reqwest::Client,
    url: &reqwest::Url,
    event_tx: &UnboundedSender<Event>,
) -> Result<()> {
    let response = http
        .get(url.clone())
        .header(reqwest::header::ACCEPT, "text/event-stream")
        .send()
        .await?
        .error_for_status()?;

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok());

    let is_sse = content_type
        .map(|v| v.starts_with("text/event-stream"))
        .unwrap_or(false);

    if !is_sse {
        anyhow::bail!(
            "expected text/event-stream, got {}",
            content_type.unwrap_or("<none>")
        );
    }

    if event_tx
        .send(Event::Backend(BackendEvent::Connected))
        .is_err()
    {
        return Ok(());
    }

    let events = response.bytes_stream().eventsource();
    futures::pin_mut!(events);

    while let Some(event) = events.next().await {
        let event = event?;
        match serde_json::from_str::<PlayerStatus>(&event.data) {
            Ok(status) => {
                if event_tx
                    .send(Event::Backend(BackendEvent::StatusReceived(status)))
                    .is_err()
                {
                    return Ok(());
                }
            }
            Err(error) => {
                let _ = event_tx.send(Event::Backend(BackendEvent::MalformedStatus(
                    error.to_string(),
                )));
            }
        }
    }

    Ok(())
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

pub async fn set_volume(
    http: &reqwest::Client,
    base_url: &reqwest::Url,
    volume: Volume,
) -> Result<()> {
    let url = base_url.join("/volume")?;

    http.post(url)
        .json(&SetVolume { volume })
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

impl Drop for Pl4yrsClient {
    fn drop(&mut self) {
        self.command_task.abort();
        self.sse_task.abort();
    }
}
