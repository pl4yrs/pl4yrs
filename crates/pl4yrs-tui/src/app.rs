use crate::{
    event::{AppEvent, Event, EventHandler},
    pl4yrs_client::{BackendCommand, BackendEvent, Pl4yrsClient},
};
use anyhow::{Error, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use pl4yrs_api::{
    control::Volume,
    playback::{PlayerState, PlayerStatus},
};
use ratatui::DefaultTerminal;
use std::time::Duration;
use tokio::{sync::mpsc::error::TrySendError, time::Instant};

const PENDING_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Event handler.
    pub events: EventHandler,

    pub client: Pl4yrsClient,
    pub status: Option<PlayerStatus>,
    pub last_error: Option<String>,
    pub connection_status: Option<ConnectionStatus>,

    pub pending_playback: Option<PendingPlayback>,
}

#[derive(Debug)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected(String),
}

#[derive(Debug, Copy, Clone)]
pub struct PendingPlayback {
    pub intent: PlayerState,
    pub since: Instant,
}

impl App {
    pub fn new(base_url: reqwest::Url) -> Result<App, Error> {
        let events = EventHandler::new();
        let pl4yrs_client = Pl4yrsClient::start(base_url, events.sender())?;

        Ok(App {
            running: true,
            events,
            client: pl4yrs_client,
            status: None,
            last_error: None,
            connection_status: None,
            pending_playback: None,
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    crossterm::event::Event::Key(key_event)
                        if key_event.kind == crossterm::event::KeyEventKind::Press =>
                    {
                        self.handle_key_events(key_event)?
                    }
                    _ => {}
                },
                Event::App(app_event) => match app_event {
                    AppEvent::Quit => self.quit(),
                    AppEvent::VolumeDecrease => self.volume_decrease(),
                    AppEvent::VolumeIncrease => self.volume_increase(),
                    AppEvent::TogglePlayback => self.toggle_playback(),
                },
                Event::Backend(backend_event) => match backend_event {
                    BackendEvent::Connecting => self.backend_connecting(),
                    BackendEvent::Connected => self.backend_connected(),
                    BackendEvent::Disconnected(message) => self.backend_disconnected(message),
                    BackendEvent::RequestFailed(message) => self.backend_failed(message),
                    BackendEvent::StatusReceived(status) => self.backend_update_status(status),
                    BackendEvent::MalformedStatus(message) => {
                        self.backend_malformed_status(message)
                    }
                },
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.events.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            }
            KeyCode::Char('+') => self.events.send(AppEvent::VolumeIncrease),
            KeyCode::Char('-') => self.events.send(AppEvent::VolumeDecrease),
            KeyCode::Char(' ') => self.events.send(AppEvent::TogglePlayback),
            _ => {}
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&mut self) {
        if let Some(pending) = self.pending_playback
            && pending.since.elapsed() > PENDING_COMMAND_TIMEOUT
        {
            self.pending_playback = None;
            self.last_error =
                Some("No response from player - the command may not have been applied".to_owned());
        }
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn fetch_status(&mut self) {
        self.send_backend_command(BackendCommand::FetchStatus);
    }

    fn send_backend_command(&mut self, cmd: BackendCommand) {
        match self.client.try_send(cmd) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                self.last_error =
                    Some("Too many pending requests; please try again shortly.".to_owned());
            }
            Err(TrySendError::Closed(_)) => {
                self.last_error = Some("The backend command worker is unavailable.".to_owned());
            }
        }
    }

    pub fn backend_connecting(&mut self) {
        self.connection_status = Some(ConnectionStatus::Connecting);
    }

    pub fn backend_connected(&mut self) {
        self.connection_status = Some(ConnectionStatus::Connected);
    }

    pub fn backend_disconnected(&mut self, message: String) {
        self.connection_status = Some(ConnectionStatus::Disconnected(message));
    }

    pub fn backend_failed(&mut self, message: String) {
        self.pending_playback = None;
        self.last_error = Some(message);
    }

    pub fn backend_update_status(&mut self, status: PlayerStatus) {
        if let Some(pending) = self.pending_playback {
            let satisfied = match pending.intent {
                PlayerState::Playing => {
                    matches!(status.state, PlayerState::Playing | PlayerState::Buffering)
                }
                other => status.state == other,
            };
            if satisfied || status.state == pending.intent {
                self.pending_playback = None;
            }
        }
        self.status = Some(status);
        self.last_error = None;
    }

    pub fn backend_malformed_status(&mut self, message: String) {
        self.last_error = Some(message);
    }

    pub fn volume_increase(&mut self) {
        let Some(output) = self.status.as_ref().and_then(|s| s.output.as_ref()) else {
            return;
        };
        let volume = output.group_volume.unwrap_or(output.volume);
        let volume = Volume::from_clamped(volume.saturating_add(1));
        self.set_volume(volume);
    }

    pub fn volume_decrease(&mut self) {
        let Some(output) = self.status.as_ref().and_then(|s| s.output.as_ref()) else {
            return;
        };
        let volume = output.group_volume.unwrap_or(output.volume);
        let volume = Volume::from_clamped(volume.saturating_sub(1));
        self.set_volume(volume);
    }

    fn set_volume(&mut self, volume: Volume) {
        self.send_backend_command(BackendCommand::SetVolume(volume));
    }

    pub fn toggle_playback(&mut self) {
        if self.pending_playback.is_some() {
            return;
        }

        let (cmd, intent) = match self.status.as_ref().map(|s| s.state) {
            Some(PlayerState::Playing) | Some(PlayerState::Buffering) => {
                (BackendCommand::Pause, PlayerState::Paused)
            }
            Some(_) => (BackendCommand::Play, PlayerState::Playing),
            None => return,
        };

        self.pending_playback = Some(PendingPlayback {
            intent,
            since: Instant::now(),
        });
        self.send_backend_command(cmd);
    }
}
