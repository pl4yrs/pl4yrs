use crate::{
    event::{AppEvent, Event, EventHandler},
    pl4yrs_client::{BackendCommand, BackendEvent, Pl4yrsClient},
};
use anyhow::{Error, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use pl4yrs_api::playback::PlayerStatus;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc::error::TrySendError;

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Counter.
    pub counter: u8,
    /// Event handler.
    pub events: EventHandler,

    pub client: Pl4yrsClient,
    pub status: Option<PlayerStatus>,
    pub last_error: Option<String>,
}

impl App {
    pub fn new(base_url: reqwest::Url) -> Result<App, Error> {
        let events = EventHandler::new();
        let pl4yrs_client = Pl4yrsClient::start(base_url, events.sender())?;

        Ok(App {
            running: true,
            counter: 0,
            events: events,
            client: pl4yrs_client,
            status: None,
            last_error: None,
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
                    AppEvent::Increment => self.increment_counter(),
                    AppEvent::Decrement => self.decrement_counter(),
                    AppEvent::FetchStatus => self.fetch_status(),
                    AppEvent::Quit => self.quit(),
                },
                Event::Backend(backend_event) => match backend_event {
                    BackendEvent::Connecting => self.backend_connecting(),
                    BackendEvent::Connected => self.backend_connected(),
                    BackendEvent::Disconnected(message) => self.backend_disconnected(message),
                    BackendEvent::RequestFailed(message) => self.backend_failed(message),
                    BackendEvent::StatusReceived(status) => self.backend_update_status(status),
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
            KeyCode::Right => self.events.send(AppEvent::Increment),
            KeyCode::Left => self.events.send(AppEvent::Decrement),
            KeyCode::Char('s' | 'S') => self.events.send(AppEvent::FetchStatus),
            _ => {}
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn increment_counter(&mut self) {
        self.counter = self.counter.saturating_add(1);
    }

    pub fn decrement_counter(&mut self) {
        self.counter = self.counter.saturating_sub(1);
    }

    pub fn fetch_status(&mut self) {
        match self.client.try_send(BackendCommand::FetchStatus) {
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

    pub fn backend_connecting(&mut self) {}

    pub fn backend_connected(&mut self) {}

    pub fn backend_disconnected(&mut self, _message: String) {}

    pub fn backend_failed(&mut self, message: String) {
        self.last_error = Some(message);
    }

    pub fn backend_update_status(&mut self, status: PlayerStatus) {
        self.status = Some(status);
        self.last_error = None;
    }
}
