use super::{Playback, PlayerCommand};
use std::collections::VecDeque;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Effect {
    StartFile(PathBuf),
    PauseOutput,
    ResumeOutput,
    StopOutput,
    Error(String),
}

pub struct Outcome {
    pub next: Playback,
    pub effects: Vec<Effect>,
}

pub fn transition(state: &Playback, cmd: &PlayerCommand, queue: &mut VecDeque<PathBuf>) -> Outcome {
    use Playback::*;

    match (cmd, state) {
        (PlayerCommand::Enqueue { track: path }, _) => {
            queue.push_back(path.clone());
            Outcome {
                next: state.clone(),
                effects: vec![],
            }
        }
        (PlayerCommand::Play, _) => match queue.pop_front() {
            Some(track) => Outcome {
                next: Loading {
                    track: track.clone(),
                },
                effects: vec![Effect::StartFile(track)],
            },
            None => Outcome {
                next: Stopped,
                effects: vec![Effect::Error("queue empty".into())],
            },
        },
        (PlayerCommand::Pause, Playing { track }) => Outcome {
            next: Paused {
                track: track.clone(),
            },
            effects: vec![Effect::PauseOutput],
        },
        (PlayerCommand::Resume, Paused { track }) => Outcome {
            next: Playing {
                track: track.clone(),
            },
            effects: vec![Effect::ResumeOutput],
        },
        (PlayerCommand::Stop, _) => Outcome {
            next: Stopped,
            effects: vec![Effect::StopOutput],
        },
        (cmd, s) => Outcome {
            next: s.clone(),
            effects: vec![Effect::Error(format!("{cmd:?} invalid in {s:?}"))],
        },
    }
}
