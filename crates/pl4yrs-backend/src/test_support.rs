use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

pub const STATUS_RESPONSE: &str = "\
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

/// Default command table: everything succeeds with `ok`, `status` returns STATUS_RESPONSE.
pub fn default_handler(cmd: &str) -> String {
    match cmd {
        "status" => STATUS_RESPONSE.to_owned(),
        "play" | "pause" => "ok".into(),
        c if c.starts_with("vol ") => "ok".into(),
        _ => "error".into(),
    }
}

/// Spawns a fake sendspin daemon on a unix socket inside `dir`.
/// `handler` maps a command line (trimmed) to its response.
/// Returns the socket path to pass to `SendSpinService::new`.
pub fn mock_sendspin(
    dir: &TempDir,
    handler: impl Fn(&str) -> String + Send + Sync + 'static,
) -> PathBuf {
    let sock_path = dir.path().join("control.sock");
    let listener = UnixListener::bind(&sock_path).unwrap();
    let handler = Arc::new(handler);

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let handler = handler.clone();
            tokio::spawn(async move {
                let (reader, mut writer) = stream.into_split();
                let mut line = String::new();
                if BufReader::new(reader).read_line(&mut line).await.is_err() {
                    return;
                }
                let _ = writer.write_all(handler(line.trim()).as_bytes()).await;
                let _ = writer.shutdown().await;
            });
        }
    });

    sock_path
}
