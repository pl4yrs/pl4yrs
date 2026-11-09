use super::fsm::{Effect, transition};
use super::{Playback, PlayerCommand, PlayerEvent};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, error, info};

const TICK: Duration = Duration::from_millis(250);

#[derive(Clone)]
pub struct PlayerHandle {
    cmd: mpsc::Sender<PlayerCommand>,
    events: broadcast::Sender<PlayerEvent>,
}

impl PlayerHandle {
    pub fn send(&self, cmd: PlayerCommand) {
        let _ = self.cmd.send(cmd);
    }
    pub fn subscribe(&self) -> broadcast::Receiver<PlayerEvent> {
        self.events.subscribe()
    }
}

pub fn spawn() -> PlayerHandle {
    dbg!("Spawning player thread");
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (events, _) = broadcast::channel(64);
    let handle = PlayerHandle {
        cmd: cmd_tx,
        events: events.clone(),
    };
    std::thread::spawn(move || player_loop(cmd_rx, events));
    handle
}

fn player_loop(rx: Receiver<PlayerCommand>, events: broadcast::Sender<PlayerEvent>) {
    let sink = rodio::DeviceSinkBuilder::from_default_device()
        .and_then(|b| b.open_stream())
        .expect("open default output device");
    let player = rodio::Player::connect_new(sink.mixer());

    let mut state = Playback::Stopped;
    let mut queue: VecDeque<PathBuf> = VecDeque::new();

    dbg!("Initializing player loop");
    loop {
        let cmd = match rx.recv_timeout(TICK) {
            Ok(c) => Some(c),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => Some(PlayerCommand::Shutdown),
        };

        if cmd == Some(PlayerCommand::Shutdown) {
            info!("Received shutdown, exiting");
            break;
        }

        if cmd.is_some() {
            debug!(?cmd, "player command");
        }

        if let Playback::Playing { .. } = &state {
            if player.empty() {
                state = Playback::Stopped;
                let _ = events.send(PlayerEvent::StateChanged {
                    playback: state.clone(),
                });
            } else {
                let _ = events.send(PlayerEvent::Position {
                    pos: player.get_pos().as_millis() as u64,
                });
            }
        }

        let Some(cmd) = cmd else { continue };

        let outcome = transition(&state, &cmd, &mut queue);
        debug!(from = ?state, to = ?outcome.next, "transition");
        state = outcome.next;
        for effect in outcome.effects {
            state = apply(effect, &player, &events, state);
        }
    }
}

fn apply(
    effect: Effect,
    player: &rodio::Player,
    events: &broadcast::Sender<PlayerEvent>,
    state: Playback,
) -> Playback {
    use std::io::BufReader;
    debug!("Applying effect");
    match effect {
        Effect::StartFile(path) => {
            let _span = tracing::info_span!("track", path = %path.display()).entered();
            debug!("opening file");
            let result = std::fs::File::open(&path)
                .map(BufReader::new)
                .map_err(|e| format!("open {}: {e}", path.display()))
                .and_then(|r| rodio::Decoder::try_from(r).map_err(|e| e.to_string()));
            match result {
                Ok(source) => {
                    info!("playing");
                    player.append(source);
                    let next = Playback::Playing { track: path };
                    let _ = events.send(PlayerEvent::StateChanged {
                        playback: next.clone(),
                    });
                    next
                }
                Err(e) => {
                    error!("could not open file");
                    let _ = events.send(PlayerEvent::Error { message: e });
                    Playback::Stopped
                }
            }
        }
        Effect::PauseOutput => {
            dbg!("pausing player at {}", player.get_pos());
            player.pause();
            state
        }
        Effect::ResumeOutput => {
            dbg!("resuming playback at {}", player.get_pos());
            player.play();
            state
        }
        Effect::StopOutput => {
            dbg!("stopping player at {}", player.get_pos());
            player.stop();
            state
        }
        Effect::Error(msg) => {
            error!(msg, "player error");
            let _ = events.send(PlayerEvent::Error { message: msg });
            state
        }
    }
}
