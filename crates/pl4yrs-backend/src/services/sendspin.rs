use anyhow::Result;
use pl4yrs_api::control::Volume;
use serde::Serialize;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::watch;
use tracing::{debug, error, warn};
use valuable::Valuable;

const SENDSPIN_STATUS_POLL_INTERVAL_MS: u64 = 200;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SendSpinStatus {
    pub name: String,
    pub client_id: String,
    pub server: String,
    pub trust: String,
    pub pairing: String,
    pub state: PlaybackState,
    pub stream: String,
    pub track: String,
    pub position: Option<Position>,
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

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Position {
    pub position: Duration,
    pub duration: Option<Duration>,
    pub estimated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Valuable)]
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
    pub status_rx: watch::Receiver<SendSpinStatus>,
    status_tx: Arc<watch::Sender<SendSpinStatus>>,
}

// Implementing the sendspin-cpp-cli protocol
// Details of the protocol are available here https://github.com/Sendspin/sendspin-cpp-cli/wiki/Controlling-the-Player
impl SendSpinService {
    pub fn new(socket_path: PathBuf, config_file: PathBuf) -> Self {
        let (status_tx, status_rx) = watch::channel(SendSpinStatus::default());
        Self {
            socket_path,
            config_file,
            status_rx,
            status_tx: Arc::new(status_tx),
        }
    }

    pub fn start_polling(&self) {
        let socket_path = self.socket_path.clone();
        let tx = self.status_tx.clone();

        tokio::spawn(async move {
            let mut ticker =
                tokio::time::interval(Duration::from_millis(SENDSPIN_STATUS_POLL_INTERVAL_MS));
            loop {
                ticker.tick().await;
                match Self::fetch_status(&socket_path).await {
                    Ok(status) => {
                        if *tx.borrow() != status {
                            debug!(status = ?status, "Got new status response");
                            let _ = tx.send(status);
                        }
                    }
                    Err(e) => warn!("sendspin poll failed: {e}"),
                }
            }
        });
    }

    async fn fetch_status(socket_path: &Path) -> Result<SendSpinStatus, SendSpinError> {
        let response = Self::send_command(socket_path, "status").await?;
        Self::parse_status_response(&response)
    }

    // This actually sets the group volume. There doesn't seem to be a way to set this specific
    // player volume via the exposed interface
    pub async fn set_volume(&self, volume: Volume) -> Result<(), SendSpinError> {
        self.run_command(&format!("vol {}", volume.get())).await
    }

    pub async fn play(&self) -> Result<(), SendSpinError> {
        self.run_command("play").await
    }

    pub async fn pause(&self) -> Result<(), SendSpinError> {
        self.run_command("pause").await
    }

    async fn run_command(&self, cmd: &str) -> Result<(), SendSpinError> {
        let response = Self::send_command(&self.socket_path, cmd).await?;
        Self::parse_ok_response(&response)?;
        self.refresh_status().await;
        Ok(())
    }

    async fn send_command(socket_path: &Path, cmd: &str) -> Result<String, SendSpinError> {
        let mut stream = UnixStream::connect(socket_path)
            .await
            .map_err(SendSpinError::Connect)?;

        stream.write_all(format!("{cmd}\n").as_bytes()).await?;

        let mut response = String::new();
        stream.read_to_string(&mut response).await?;
        Ok(response)
    }

    async fn refresh_status(&self) {
        match Self::fetch_status(&self.socket_path).await {
            Ok(status) => {
                let _ = self.status_tx.send(status);
            }
            Err(e) => warn!("sendspin status refresh failed: {e}"),
        }
    }

    pub fn status(&self) -> SendSpinStatus {
        self.status_rx.borrow().clone()
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
            .filter(|x| !x.starts_with('#') && !x.is_empty())
        {
            debug!(line = line, "processing config line");
            if let Some((key, value)) = line.split_once("=") {
                match key.trim() {
                    "name" => config.name = value.trim().to_string(),
                    "manufacturer" => config.manufacturer = value.trim().to_string(),
                    "product_name" => config.product_name = value.trim().to_string(),
                    "output" => config.output = value.trim().to_string(),
                    "input" => config.input = value.trim().to_string(),
                    "buffer-ms" => config.buffer_ms = value.trim().parse().unwrap(), // XXX should we return SendSpinError::ConfigInvalid(line))?
                    "audio-format" => config.audio_format = value.trim().to_string(),
                    "port" => config.port = value.trim().parse().unwrap(),
                    "server" => config.server = value.trim().to_string(),
                    "output-delay" => config.output_delay = value.trim().parse().unwrap(),
                    "allow-unpaired" => config.allow_unpaired = Self::parse_optional_bool(value),
                    "fixed-volume" => config.fixed_volume = Self::parse_optional_bool(value),
                    "log-level" => config.log_level = value.trim().to_string(),
                    "control-socket" => config.control_socket = value.trim().to_string(),
                    _ => {
                        error!("Unknown value: {}", key.trim());
                        return Err(SendSpinError::ConfigInvalid(line));
                    }
                }
            }
        }

        Ok(config)
    }

    fn parse_status_response(response: &str) -> Result<SendSpinStatus, SendSpinError> {
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
                    "position" => {
                        status.position = Self::parse_optional_position(value).map(
                            |(position, duration, estimated)| Position {
                                position,
                                duration,
                                estimated,
                            },
                        )
                    }
                    "group volume" => status.group_volume = Self::parse_optional_u8(value),
                    "repeat" => status.repeat = Self::parse_optional_toggle(value),
                    "shuffle" => status.shuffle = Self::parse_optional_toggle(value),
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

    fn parse_ok_response(response: &str) -> Result<(), SendSpinError> {
        match response.trim() {
            "ok" => Ok(()),
            "" => Err(SendSpinError::MalformedResponse),
            other => {
                error!("command failed: {}", other);
                Err(SendSpinError::CommandFailed)
            }
        }
    }

    fn parse_state(value: &str) -> PlaybackState {
        match value {
            "playing" => PlaybackState::Playing,
            "paused" => PlaybackState::Paused,
            "stopped" => PlaybackState::Stopped,
            _ => PlaybackState::Unknown,
        }
    }

    fn parse_optional_position(value: &str) -> Option<(Duration, Option<Duration>, bool)> {
        let (times, estimated) = match value.strip_suffix(" (estimated)") {
            Some(t) => (t, true),
            None => (value, false),
        };
        let (pos, dur) = times.split_once(" / ")?;
        let position = Self::parse_clock(pos)?;
        let duration = match dur {
            "unknown" => None,
            d => Some(Self::parse_clock(d)?).filter(|d| !d.is_zero()),
        };
        Some((position, duration, estimated))
    }

    fn parse_clock(value: &str) -> Option<Duration> {
        let mut total = Duration::ZERO;
        for (unit, part) in value.trim().rsplit(':').enumerate() {
            let n: u64 = part.parse().ok()?;
            total += match unit {
                0 => Duration::from_secs(n),
                1 => Duration::from_mins(n),
                2 => Duration::from_hours(n),
                _ => return None,
            };
        }
        Some(total)
    }

    fn parse_optional_toggle(value: &str) -> Option<bool> {
        match value {
            "on" => Some(true),
            "off" => Some(false),
            "unknown" => None,
            _ => {
                warn!(value, "Invalid value for a toggle");
                None
            }
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
        match value.trim() {
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
    #[error("invalid sendspin configuration")]
    ConfigInvalid(String),
    #[error("sendspin command failed")]
    CommandFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_response_accepts_ok() {
        for input in ["ok", "ok\n", " ok ", "\nok\n", "ok\r\n"] {
            assert!(
                SendSpinService::parse_ok_response(input).is_ok(),
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn ok_response_rejects_empty_as_malformed() {
        for input in ["", "\n", "   "] {
            assert!(
                matches!(
                    SendSpinService::parse_ok_response(input),
                    Err(SendSpinError::MalformedResponse)
                ),
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn ok_response_rejects_failures_as_command_failed() {
        for input in ["error", "invalid volume", "OK", "ok\nextra"] {
            assert!(
                matches!(
                    SendSpinService::parse_ok_response(input),
                    Err(SendSpinError::CommandFailed)
                ),
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn state_parses_known_values() {
        assert_eq!(
            SendSpinService::parse_state("playing"),
            PlaybackState::Playing
        );
        assert_eq!(
            SendSpinService::parse_state("paused"),
            PlaybackState::Paused
        );
        assert_eq!(
            SendSpinService::parse_state("stopped"),
            PlaybackState::Stopped
        );
    }

    #[test]
    fn state_defaults_to_unknown() {
        for input in ["buffering", "", "PLAYING", "idle", "IDontKnowWhatImDoing"] {
            assert_eq!(
                SendSpinService::parse_state(input),
                PlaybackState::Unknown,
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn clock_parses_ss_mm_ss_hh_mm_ss() {
        assert_eq!(
            SendSpinService::parse_clock("42"),
            Some(Duration::from_secs(42))
        );
        assert_eq!(
            SendSpinService::parse_clock("2:42"),
            Some(Duration::from_secs(162))
        );
        assert_eq!(
            SendSpinService::parse_clock("1:05:42"),
            Some(Duration::from_secs(3942))
        );
    }

    #[test]
    fn clock_trims_whitespaces() {
        assert_eq!(
            SendSpinService::parse_clock("  42  "),
            Some(Duration::from_secs(42))
        );
    }

    #[test]
    fn clock_rejects_too_invalid_strings() {
        for input in ["1:02:03:09", "abc", "12:xa", "-2"] {
            assert_eq!(
                SendSpinService::parse_clock(input),
                None,
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn position_parses_position_and_duration() {
        let (pos, dur, est) = SendSpinService::parse_optional_position("1:23 / 4:42").unwrap();
        assert_eq!(pos, Duration::from_secs(83));
        assert_eq!(dur, Some(Duration::from_secs(282)));
        assert!(!est);
    }

    #[test]
    fn position_parses_estimated_suffix() {
        let (_pos, _dur, est) =
            SendSpinService::parse_optional_position("1:23 / 4:42 (estimated)").unwrap();
        assert!(est);
    }

    #[test]
    fn position_unknown_duration_is_none() {
        let (pos, dur, _) = SendSpinService::parse_optional_position("1:23 / unknown").unwrap();
        assert_eq!(pos, Duration::from_secs(83));
        assert_eq!(dur, None);
    }

    #[test]
    fn position_zero_duration_is_none() {
        let (pos, dur, _) = SendSpinService::parse_optional_position("1:23 / 0:00").unwrap();
        assert_eq!(pos, Duration::from_secs(83));
        assert_eq!(dur, None);
    }

    #[test]
    fn position_rejects_invalid_strings() {
        for input in ["1:23", "abc / 1:42", "1:23 / xa", ""] {
            assert_eq!(
                SendSpinService::parse_optional_position(input),
                None,
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn toggle_parses_on_off_unknown() {
        assert_eq!(SendSpinService::parse_optional_toggle("on"), Some(true));
        assert_eq!(SendSpinService::parse_optional_toggle("off"), Some(false));
        assert_eq!(SendSpinService::parse_optional_toggle("unknown"), None);
        assert_eq!(SendSpinService::parse_optional_toggle("yes"), None);
    }

    #[test]
    fn optional_u8_parses_number_rejects_garbage() {
        assert_eq!(SendSpinService::parse_optional_u8("42"), Some(42));
        assert_eq!(SendSpinService::parse_optional_u8("300"), None);
        assert_eq!(SendSpinService::parse_optional_u8("unknown"), None);
        assert_eq!(SendSpinService::parse_optional_u8("xyz"), None);
    }

    #[test]
    fn optional_bool_parses_true_false() {
        assert_eq!(SendSpinService::parse_optional_bool("true"), Some(true));
        assert_eq!(SendSpinService::parse_optional_bool("false"), Some(false));
        assert_eq!(SendSpinService::parse_optional_bool("yes"), None);
    }

    #[test]
    fn volume_takes_first_token_defaults_zero() {
        assert_eq!(SendSpinService::parse_volume("42 something"), 42);
        assert_eq!(SendSpinService::parse_volume("43"), 43);
        assert_eq!(SendSpinService::parse_volume("fgh "), 0);
        assert_eq!(SendSpinService::parse_volume("    "), 0);
        assert_eq!(SendSpinService::parse_volume(""), 0);
    }

    #[test]
    fn delay_strips_ms_defaults_zero() {
        assert_eq!(SendSpinService::parse_delay("42 ms"), 42);
        assert_eq!(SendSpinService::parse_delay("314"), 314);
        assert_eq!(SendSpinService::parse_delay("fgh "), 0);
        assert_eq!(SendSpinService::parse_delay("    "), 0);
        assert_eq!(SendSpinService::parse_delay(""), 0);
    }

    #[test]
    fn status_response_parses_all_fiels() {
        let response = "\
name: Upper Toilet
client id: pl4yrs-something
server: Music Assistant (ma.local)
trust: full
pairing: paired
state: playing
stream: flac
track: No Stairway to Heaven
position: 1:23 / 42:18 (estimated)
group volume: 42
repeat: on
shuffle: unknown
player volume: 65 fixed
output: plughw:CARD=DigiAMP,DEV=0
output delay: 50 ms
note: one coffee please!";
        let status = SendSpinService::parse_status_response(response).unwrap();

        assert_eq!(status.name, "Upper Toilet");
        assert_eq!(status.client_id, "pl4yrs-something");
        assert_eq!(status.server, "Music Assistant (ma.local)");
        assert_eq!(status.trust, "full");
        assert_eq!(status.pairing, "paired");
        assert_eq!(status.state, PlaybackState::Playing);
        assert_eq!(status.stream, "flac");
        assert_eq!(status.track, "No Stairway to Heaven");

        let position = status.position.unwrap();
        assert_eq!(position.position, Duration::from_secs(83));
        assert_eq!(position.duration, Some(Duration::from_secs(2538)));
        assert!(position.estimated);

        assert_eq!(status.group_volume, Some(42));
        assert_eq!(status.repeat, Some(true));
        assert_eq!(status.shuffle, None);
        assert_eq!(status.player_volume, 65);
        assert_eq!(status.output, "plughw:CARD=DigiAMP,DEV=0");
        assert_eq!(status.output_delay_ms, 50);
        assert_eq!(status.note, "one coffee please!");
    }

    #[test]
    fn status_response_unknown_key_is_malformed() {
        let err = SendSpinService::parse_status_response("garbage key: some value").unwrap_err();
        assert!(matches!(err, SendSpinError::MalformedResponse));
    }

    #[test]
    fn status_response_skips_lines_missing_colon() {
        let status = SendSpinService::parse_status_response(
            "name: pl4yrs\ngarbage line without separator\n",
        )
        .unwrap();
        assert_eq!(status.name, "pl4yrs");
    }

    #[test]
    fn status_response_empty_input_returns_defaul() {
        let status = SendSpinService::parse_status_response("").unwrap();
        assert_eq!(status, SendSpinStatus::default());
    }

    #[tokio::test]
    async fn config_parses_known_keys_skips_comments() {
        let conf = "\
# initial comment
name = testplayer
output = sysdefault
buffer-ms = 200
audio-format = pcm:48000:24:2
port = 9000
# middle comment
server = ma.local
output-delay = 10
allow-unpaired = true
fixed-volume = false
log-level = debug
control-socket = /tmp/control.sock

# trailing comment
";
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), conf).unwrap();

        let service = SendSpinService::new(
            PathBuf::from("/tmp/not-a-real.sock"),
            file.path().to_path_buf(),
        );
        let config = service.get_config().await.unwrap();

        assert_eq!(config.name, "testplayer");
        assert_eq!(config.output, "sysdefault");
        assert_eq!(config.buffer_ms, 200);
        assert_eq!(config.port, 9000);
        assert_eq!(config.server, "ma.local");
        assert_eq!(config.output_delay, 10);
        assert_eq!(config.allow_unpaired, Some(true));
        assert_eq!(config.fixed_volume, Some(false));
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.control_socket, "/tmp/control.sock");
    }

    #[tokio::test]
    async fn config_rejects_unknown_key() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "garbage-dump = 1\n").unwrap();

        let service = SendSpinService::new(
            PathBuf::from("/tmp/not-a-real.sock"),
            file.path().to_path_buf(),
        );
        let config = service.get_config().await;
        assert!(matches!(config, Err(SendSpinError::ConfigInvalid(_))));
    }
}
