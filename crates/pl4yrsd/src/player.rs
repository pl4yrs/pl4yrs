pub mod engine;
pub mod fsm;

use enum_display::EnumDisplay;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, EnumDisplay)]
pub enum PlayerCommand {
    #[display("Queuing track: {track:?}")]
    Enqueue {
        track: PathBuf,
    },
    Play,
    Pause,
    Resume,
    Stop,
    Shutdown,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PlayerEvent {
    StateChanged { playback: Playback },
    Position { pos: u64 },
    Error { message: String },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Playback {
    Stopped,
    Loading { track: PathBuf },
    Playing { track: PathBuf },
    Paused { track: PathBuf },
}
