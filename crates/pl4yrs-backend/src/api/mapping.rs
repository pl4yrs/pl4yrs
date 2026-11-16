use crate::services::sendspin::{PlaybackState, SendSpinStatus};
use pl4yrs_api::playback::{
    MediaItem, PlaybackSource, PlayerOutput, PlayerState, PlayerStatus, Progress,
};

impl From<&SendSpinStatus> for PlayerStatus {
    fn from(status: &SendSpinStatus) -> Self {
        let mut player_status = pl4yrs_api::playback::PlayerStatus {
            source: Some(PlaybackSource::Sendspin {
                server: status.server.clone(),
            }),
            state: match status.state {
                PlaybackState::Unknown => PlayerState::Unknown,
                PlaybackState::Playing => PlayerState::Playing,
                PlaybackState::Paused => PlayerState::Paused,
                PlaybackState::Stopped => PlayerState::Stopped,
            },
            output: Some(PlayerOutput {
                volume: status.player_volume,
                group_volume: status.group_volume,
                device: status.output.clone(),
                delay_ms: status.output_delay_ms,
            }),
            ..Default::default()
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
