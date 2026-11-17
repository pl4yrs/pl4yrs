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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::services::sendspin::{PlaybackState, Position};
    use pl4yrs_api::playback::PlayerState;

    #[test]
    fn map_playback_states() {
        let cases = [
            (PlaybackState::Playing, PlayerState::Playing),
            (PlaybackState::Paused, PlayerState::Paused),
            (PlaybackState::Stopped, PlayerState::Stopped),
            (PlaybackState::Unknown, PlayerState::Unknown),
        ];
        for (input, expected) in cases {
            let status = SendSpinStatus {
                state: input,
                ..Default::default()
            };
            assert_eq!(
                PlayerStatus::from(&status).state,
                expected,
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn maps_output_and_source() {
        let status = SendSpinStatus {
            server: "ma.local".into(),
            player_volume: 42,
            group_volume: Some(42),
            output: "somedevice".into(),
            output_delay_ms: 314,
            ..Default::default()
        };

        let status = PlayerStatus::from(&status);

        match status.source {
            Some(PlaybackSource::Sendspin { ref server }) => assert_eq!(server, "ma.local"),
            other => panic!("expected Sendspin source, got {other:?}"),
        }
    }

    #[test]
    fn map_position_to_progress() {
        let status = SendSpinStatus {
            position: Some(Position {
                position: Duration::from_secs(84),
                duration: Some(Duration::from_secs(159)),
                estimated: true,
            }),
            ..Default::default()
        };

        let progress = PlayerStatus::from(&status).progress.unwrap();
        assert_eq!(progress.position, Duration::from_secs(84));
        assert_eq!(progress.duration, Some(Duration::from_secs(159)));
        assert!(progress.estimated);
    }

    #[test]
    fn missing_position_yields_no_progress() {
        assert!(
            PlayerStatus::from(&SendSpinStatus::default())
                .progress
                .is_none()
        );
    }

    #[test]
    fn maps_track_to_media_title() {
        let status = SendSpinStatus {
            track: "No Stairway to Heaven".into(),
            ..Default::default()
        };
        assert_eq!(
            PlayerStatus::from(&status).media.unwrap().title,
            "No Stairway to Heaven"
        );
    }
}
