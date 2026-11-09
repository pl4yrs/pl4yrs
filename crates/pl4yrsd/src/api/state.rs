use crate::player::engine::PlayerHandle;

#[derive(Clone)]
pub struct AppState {
    pub player: PlayerHandle,
}

impl AppState {
    pub fn new(player: PlayerHandle) -> Self {
        AppState { player }
    }
}
