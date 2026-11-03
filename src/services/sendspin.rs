use anyhow::Result;
use serde::Serialize;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tracing::{debug, error};

#[derive(Debug, Clone, Serialize)]
pub struct SendSpinStatus {
    pub name: String,
    pub client_id: String,
    pub server: String,
    pub trust: String,
    pub pairing: String,
    pub state: PlaybackState,
    pub stream: String,
    pub track: String,
    pub position: Option<u64>,
    pub group_volume: Option<u8>,
    pub repeat: Option<bool>,
    pub shuffle: Option<bool>,
    pub player_volume: u8,
    pub output: String,
    pub output_delay_ms: u64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SendSpinConfig {
    pub name: String,
    manufacturer: String,
    product_name: String,
    pub output: String,
    input: String,
    pub buffer_ms: u16,
    pub audio_format: String,
    port: u16,
    server: String,
    pub output_delay: u16,
    allow_unpaired: Option<bool>,
    fixed_volume: Option<bool>,
    // pairing_code: String,
    // hook_start: String,
    // hook_stop: String,
    log_level: String,
    control_socket: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum PlaybackState {
    Unknown,
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone)]
pub struct SendSpinService {
    socket_path: PathBuf,
    config_file: PathBuf,
}

impl SendSpinService {
    pub fn new(socket_path: PathBuf, config_file: PathBuf) -> Self {
        Self {
            socket_path: socket_path,
            config_file: config_file,
        }
    }

    pub async fn get_status(&self) -> Result<SendSpinStatus, SendSpinError> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(SendSpinError::Connect)?;

        stream.write_all(b"status\n").await?;

        let mut response = String::new();
        stream.read_to_string(&mut response).await?;
        debug!(response, "Got Status response");

        self.parse_status_response(&response)
    }

    pub async fn get_config(&self) -> Result<SendSpinConfig, SendSpinError> {
        let mut config = SendSpinConfig::default();

        debug!(
            "Opening config file: {}",
            self.config_file.to_string_lossy()
        );

        let lines = Self::read_lines(&self.config_file)
            .map_err(SendSpinError::ConfigAccess)
            .unwrap();

        for line in lines
            .map_while(Result::ok)
            .filter(|x| !x.starts_with('#') && x != "")
        {
            debug!(line = line, "processing config line");
            if let Some((key, value)) = line.split_once("=") {
                match key.trim() {
                    "name" => config.name = value.to_string(),
                    "manufacturer" => config.manufacturer = value.to_string(),
                    "product_name" => config.product_name = value.to_string(),
                    "output" => config.output = value.to_string(),
                    "input" => config.input = value.to_string(),
                    "buffer-ms" => config.buffer_ms = value.trim().parse().unwrap(), // XXX should we return SendSpinError::ConfigInvalid(line))?
                    "audio-format" => config.audio_format = value.to_string(),
                    "port" => config.port = value.trim().parse().unwrap(),
                    "server" => config.server = value.to_string(),
                    "output-delay" => config.output_delay = value.trim().parse().unwrap(),
                    "allow-unpaired" => config.allow_unpaired = Self::parse_optional_bool(value),
                    "fixed-volume" => config.fixed_volume = Self::parse_optional_bool(value),
                    "log-level" => config.log_level = value.to_string(),
                    "control-socket" => config.control_socket = value.to_string(),
                    _ => {
                        error!("Unknown value: {}", key.trim());
                        return Err(SendSpinError::ConfigInvalid(line));
                    }
                }
            }
        }

        Ok(config)
    }

    fn parse_status_response(&self, response: &str) -> Result<SendSpinStatus, SendSpinError> {
        let mut status = SendSpinStatus::default();
        let lines: Vec<&str> = response.lines().collect();

        for line in lines {
            if let Some((key, value)) = line.split_once(": ") {
                match key.trim() {
                    "name" => status.name = value.to_string(),
                    "client id" => status.client_id = value.to_string(),
                    "server" => status.server = value.to_string(),
                    "trust" => status.trust = value.to_string(),
                    "pairing" => status.pairing = value.to_string(),
                    "state" => status.state = Self::parse_state(value),
                    "stream" => status.stream = value.to_string(),
                    "track" => status.track = value.to_string(),
                    "position" => status.position = Self::parse_optional_u64(value),
                    "group volume" => status.group_volume = Self::parse_optional_u8(value),
                    "repeat" => status.repeat = Self::parse_optional_bool(value),
                    "shuffle" => status.shuffle = Self::parse_optional_bool(value),
                    "player volume" => status.player_volume = Self::parse_volume(value),
                    "output" => status.output = value.to_string(),
                    "output delay" => status.output_delay_ms = Self::parse_delay(value),
                    "note" => status.note = value.to_string(),
                    _ => {
                        error!("Unknown value: {}", key.trim());
                        return Err(SendSpinError::MalformedResponse);
                    }
                }
            }
        }
        Ok(status)
    }

    fn parse_state(value: &str) -> PlaybackState {
        match value {
            "playing" => PlaybackState::Playing,
            "paused" => PlaybackState::Paused,
            "stopped" => PlaybackState::Stopped,
            _ => PlaybackState::Unknown,
        }
    }

    fn parse_optional_u64(value: &str) -> Option<u64> {
        if value == "unknown" {
            None
        } else {
            value.parse().ok()
        }
    }

    fn parse_optional_u8(value: &str) -> Option<u8> {
        if value == "unknown" {
            None
        } else {
            value.parse().ok()
        }
    }

    fn parse_optional_bool(value: &str) -> Option<bool> {
        match value {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    fn parse_volume(value: &str) -> u8 {
        value
            .split(' ')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    fn parse_delay(value: &str) -> u64 {
        value.trim_end_matches(" ms").parse().unwrap_or(0)
    }

    fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
    where
        P: AsRef<Path>,
    {
        let file = File::open(filename)?;
        Ok(io::BufReader::new(file).lines())
    }
}

impl Default for SendSpinStatus {
    fn default() -> Self {
        Self {
            name: String::new(),
            client_id: String::new(),
            server: String::new(),
            trust: String::new(),
            pairing: String::new(),
            state: PlaybackState::Unknown,
            stream: String::new(),
            track: String::new(),
            position: None,
            group_volume: None,
            repeat: None,
            shuffle: None,
            player_volume: 0,
            output: String::new(),
            output_delay_ms: 0,
            note: String::new(),
        }
    }
}

impl Default for SendSpinConfig {
    fn default() -> Self {
        Self {
            name: "playrs".to_string(),
            manufacturer: "senspin-cpp-cli".to_string(),
            product_name: "sendspin-cli".to_string(),
            output: String::new(),
            input: String::new(),
            buffer_ms: 100,
            audio_format: "flac:48000:24:2,pcm:48000:24:2".to_string(),
            port: 8928,
            server: String::new(),
            output_delay: 0,
            allow_unpaired: Some(false),
            fixed_volume: Some(false),
            log_level: "info".to_string(),
            control_socket: "/run/sendspin-cli/control.sock".to_string(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SendSpinError {
    #[error("failed to connect to sendspin socket")]
    Connect(#[source] std::io::Error),
    #[error("failed to communicate with sendspin")]
    Io(#[from] std::io::Error),
    #[error("malformed response")]
    MalformedResponse,
    #[error("could not access sendspin config")]
    ConfigAccess(#[source] std::io::Error),
    #[error("invalid sendspin configuratio")]
    ConfigInvalid(String),
}
