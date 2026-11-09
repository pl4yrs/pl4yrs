use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub struct PlayerStatus {
    pub state: PlayerState,
    pub source: Option<PlaybackSource>,
    pub media: Option<MediaItem>,
    pub progress: Option<Progress>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerState {
    Idle,
    Playing,
    Paused,
    Buffering,
    Stopped,
    Unknown,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlaybackSource {
    Sendspin { server: String },
    LocalFile,
}

#[derive(Serialize, Deserialize)]
pub struct MediaItem {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<Duration>,
    pub artwork_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Progress {
    pub position: Duration,
    pub duration: Option<Duration>,
    pub estimated: bool,
}

impl Default for PlayerStatus {
    fn default() -> Self {
        PlayerStatus {
            state: PlayerState::Idle,
            source: None,
            media: None,
            progress: None,
        }
    }
}

impl Default for MediaItem {
    fn default() -> Self {
        MediaItem {
            title: String::new(),
            artist: None,
            album: None,
            duration: None,
            artwork_url: None,
        }
    }
}
