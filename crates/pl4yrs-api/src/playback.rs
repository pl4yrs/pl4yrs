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

#[derive(Default, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sendspin_source_uses_tagged_json() {
        let source = PlaybackSource::Sendspin {
            server: "Music Assistant Test".to_owned(),
        };

        let actual = serde_json::to_value(source).unwrap();

        assert_eq!(
            actual,
            json!({
                "kind": "sendspin",
                "server": "Music Assistant Test",
            })
        );
    }

    #[test]
    fn idle_status_can_omit_optional_fields() {
        let status: PlayerStatus = serde_json::from_value(json!({
            "state": "idle"
        }))
        .unwrap();

        assert!(matches!(status.state, PlayerState::Idle));
        assert!(status.source.is_none());
        assert!(status.media.is_none());
        assert!(status.progress.is_none());
    }

    #[test]
    fn track_media_can_omit_optional_fields() {
        let media: MediaItem = serde_json::from_value(json!({
            "title": "The Cure - In-Between days",
            "artist": null,
            "album": null,
            "duration": null,
            "artwork_url": null
        }))
        .unwrap();

        assert_eq!(media.title, "The Cure - In-Between days");
        assert!(media.artist.is_none());
        assert!(media.album.is_none());
        assert!(media.duration.is_none());
        assert!(media.artwork_url.is_none());
    }

    #[test]
    fn progress_duration_can_be_unknown() {
        let progress: Progress = serde_json::from_value(json!({
            "position": {
                "secs": 8612,
                "nanos": 0
            },
            "duration": null,
            "estimated": true
        }))
        .unwrap();

        assert_eq!(progress.position.as_secs(), 8612);
        assert_eq!(progress.estimated, true);
        assert!(progress.duration.is_none());
    }
}
