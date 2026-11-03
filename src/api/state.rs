use std::path::PathBuf;

use crate::services::sendspin::SendSpinService;

#[derive(Clone)]
pub struct AppState {
    pub sendspin_service: SendSpinService,
}

impl AppState {
    pub fn new(sendspin_socket: PathBuf, sendspin_config_file: PathBuf) -> Self {
        AppState {
            sendspin_service: SendSpinService::new(sendspin_socket, sendspin_config_file),
        }
    }
}
