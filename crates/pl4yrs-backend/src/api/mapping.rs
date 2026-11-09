use crate::services::sendspin::{PlaybackState, SendSpinStatus};
use pl4yrs_api::playback::{MediaItem, PlaybackSource, PlayerState, PlayerStatus, Progress};

impl From<&SendSpinStatus> for PlayerStatus {
    fn from(status: &SendSpinStatus) -> Self {
        let mut player_status = PlayerStatus::default();
        player_status.source = Some(PlaybackSource::Sendspin {
            server: status.server.clone(),
        });

        player_status.state = match status.state {
            PlaybackState::Unknown => PlayerState::Unknown,
            PlaybackState::Playing => PlayerState::Playing,
            PlaybackState::Paused => PlayerState::Paused,
            PlaybackState::Stopped => PlayerState::Stopped,
        };

        if let Some(ref position) = status.position {
            player_status.progress = Some(Progress {
                position: (position.position),
                duration: (position.duration),
                estimated: (position.estimated),
            });
        }

        player_status.media = Some(MediaItem {
            title: status.track.clone(),
            ..MediaItem::default()
        });

        player_status
    }
}
