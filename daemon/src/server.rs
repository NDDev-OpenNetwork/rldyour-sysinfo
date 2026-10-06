//! Bounded client admission and one shared sampling clock. Connecting never
//! triggers an extra hardware read; slow readers are disconnected instead of
//! blocking another client's next sample.

use crate::collector::Collector;
use crate::transport::{UnixListener, UnixStream};
use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TrySendError};
use std::time::{Duration, Instant};

const DEFAULT_INTERVAL: Duration = Duration::from_secs(5);
const MAX_INTERVAL: u64 = 60;
const REALTIME_INTERVAL: Duration = Duration::from_millis(500);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_HANDSHAKE: usize = 256;
const MAX_CLIENTS: usize = 32;
const IDLE_EXIT: Duration = Duration::from_secs(30);
const ACCEPT_STACK: usize = 64 * 1024;

pub fn serve(listener: UnixListener, activated: bool) -> io::Result<()> {
    let mut collector = Collector::new()?;
    let clients = accept_loop(listener)?;
    let mut connected = Vec::with_capacity(MAX_CLIENTS);
    let configured = interval();
    let mut line = String::with_capacity(512);
    collector.sample().encode(&mut line);
    let mut sampled_at = Instant::now();
    let mut idle_since = Some(sampled_at);

    loop {
        let wait = if connected.is_empty() {
            IDLE_EXIT.saturating_sub(idle_since.unwrap_or(sampled_at).elapsed())
        } else {
            cadence(&connected, configured).saturating_sub(sampled_at.elapsed())
        };
        match clients.recv_timeout(wait) {
            Ok(client) => {
                refresh_for_client(
                    &client,
                    &mut collector,
                    &mut sampled_at,
                    &mut line,
                    &mut connected,
                    configured,
                );
                admit(client, &mut connected, line.as_bytes());
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(io::Error::other("accept thread stopped"));
            }
        }
        // A continuously connecting peer cannot keep us draining forever.
        for _ in 1..MAX_CLIENTS {
            let Ok(client) = clients.try_recv() else {
                break;
            };
            refresh_for_client(
                &client,
                &mut collector,
                &mut sampled_at,
                &mut line,
                &mut connected,
                configured,
            );
            admit(client, &mut connected, line.as_bytes());
        }

        if !connected.is_empty() && sampled_at.elapsed() >= cadence(&connected, configured) {
            collector.sample().encode(&mut line);
            sampled_at = Instant::now();
            connected.retain_mut(|client| client.stream.write_all(line.as_bytes()).is_ok());
        }
        if connected.is_empty() {
            let since = idle_since.get_or_insert_with(Instant::now);
            if since.elapsed() >= IDLE_EXIT {
                if activated {
                    return Ok(());
                }
                // Standalone/Windows waits on the channel indefinitely; it
                // neither polls nor samples hardware when nobody is watching.
                let client = clients
                    .recv()
                    .map_err(|_| io::Error::other("accept thread stopped"))?;
                refresh_for_client(
                    &client,
                    &mut collector,
                    &mut sampled_at,
                    &mut line,
                    &mut connected,
                    configured,
                );
                admit(client, &mut connected, line.as_bytes());
                if !connected.is_empty() {
                    idle_since = None;
                }
            }
        } else {
            idle_since = None;
        }
    }
}

fn refresh_for_client(
    client: &Client,
    collector: &mut Collector,
    sampled_at: &mut Instant,
    line: &mut String,
    connected: &mut Vec<Client>,
    configured: Duration,
) {
    if connected.len() == MAX_CLIENTS {
        return;
    }
    let wanted = connected
        .iter()
        .filter_map(|client| client.requested)
        .chain(client.requested)
        .min()
        .unwrap_or(configured);
    if sampled_at.elapsed() >= wanted {
        collector.sample().encode(line);
        *sampled_at = Instant::now();
        connected.retain_mut(|client| client.stream.write_all(line.as_bytes()).is_ok());
    }
}

fn admit(mut client: Client, connected: &mut Vec<Client>, latest: &[u8]) {
    if connected.len() < MAX_CLIENTS && client.stream.write_all(latest).is_ok() {
        connected.push(client);
    }
}

fn interval() -> Duration {
    std::env::var("RLDYOUR_SYSINFO_INTERVAL")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .and_then(duration)
        .unwrap_or(DEFAULT_INTERVAL)
}

fn duration(seconds: u64) -> Option<Duration> {
    match seconds {
        0 => Some(REALTIME_INTERVAL),
        1..=MAX_INTERVAL => Some(Duration::from_secs(seconds)),
        _ => None,
    }
}

struct Client {
    stream: UnixStream,
    requested: Option<Duration>,
}

fn cadence(clients: &[Client], configured: Duration) -> Duration {
    clients
        .iter()
        .filter_map(|client| client.requested)
        .min()
        .unwrap_or(configured)
}

fn handshake(mut stream: &UnixStream) -> io::Result<Option<Duration>> {
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    let mut bytes = [0u8; MAX_HANDSHAKE];
    let mut used = 0;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        stream.set_read_timeout(Some(remaining))?;
        match stream.read(&mut bytes[used..]) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(count) => {
                used += count;
                if let Some(end) = bytes[..used].iter().position(|byte| *byte == b'\n') {
                    return Ok(std::str::from_utf8(&bytes[..end])
                        .ok()
                        .and_then(parse_interval));
                }
                if used == MAX_HANDSHAKE {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "handshake exceeds 256 bytes",
                    ));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        }
    }
}

fn parse_interval(line: &str) -> Option<Duration> {
    let object = line.trim().strip_prefix('{')?.strip_suffix('}')?;
    let (key, value) = object.split_once(':')?;
    let value = value.trim();
    if key.trim() != "\"interval\""
        || value.is_empty()
        || !value.bytes().all(|b| b.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return None;
    }
    duration(value.parse().ok()?)
}

fn accept_loop(listener: UnixListener) -> io::Result<Receiver<Client>> {
    let (sender, receiver) = mpsc::sync_channel(MAX_CLIENTS);
    std::thread::Builder::new()
        .name("accept".into())
        .stack_size(ACCEPT_STACK)
        .spawn(move || {
            loop {
                let stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => {
                        eprintln!("rldyour-sysinfod: accept: {error}");
                        return;
                    }
                };
                let Ok(requested) = handshake(&stream) else {
                    continue;
                };
                // A partial frame or WouldBlock disconnects this peer; other peers
                // never inherit its write delay, even when the socket buffer fills.
                if stream.set_nonblocking(true).is_err() {
                    continue;
                }
                match sender.try_send(Client { stream, requested }) {
                    Ok(()) | Err(TrySendError::Full(_)) => {}
                    Err(TrySendError::Disconnected(_)) => return,
                }
            }
        })?;
    Ok(receiver)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_bounded_whole_seconds_in_a_complete_object() {
        for (line, expected) in [
            (r#"{"interval":0}"#, Some(REALTIME_INTERVAL)),
            (r#"{ "interval" : 12 }"#, Some(Duration::from_secs(12))),
            (r#"{"interval":60}"#, Some(Duration::from_secs(60))),
        ] {
            assert_eq!(parse_interval(line), expected);
        }
        for line in [
            "",
            "hello",
            r#"{"interval":61}"#,
            r#"{"interval":-1}"#,
            r#"{"interval":1.5}"#,
            r#"{"interval":1e2}"#,
            r#"{"interval":00}"#,
            r#"{"interval":"5"}"#,
            r#"garbage "interval":0"#,
            r#"{"interval":3, "other":4}"#,
        ] {
            assert_eq!(parse_interval(line), None, "{line}");
        }
    }

    #[test]
    fn fastest_client_wins() {
        let make = |seconds| Client {
            stream: UnixStream::pair().unwrap().0,
            requested: duration(seconds),
        };
        assert_eq!(cadence(&[], DEFAULT_INTERVAL), DEFAULT_INTERVAL);
        assert_eq!(
            cadence(&[make(0), make(10)], DEFAULT_INTERVAL),
            REALTIME_INTERVAL
        );
        assert_eq!(
            cadence(&[make(10), make(61)], DEFAULT_INTERVAL),
            Duration::from_secs(10)
        );
    }

    #[test]
    fn oversized_handshake_is_rejected_without_growing_a_buffer() {
        let (server, mut peer) = UnixStream::pair().unwrap();
        peer.write_all(&[b'x'; MAX_HANDSHAKE]).unwrap();
        assert_eq!(
            handshake(&server).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn client_limit_is_enforced() {
        let mut clients = Vec::new();
        let mut peers = Vec::new();
        for _ in 0..MAX_CLIENTS + 1 {
            let (stream, peer) = UnixStream::pair().unwrap();
            peers.push(peer);
            admit(
                Client {
                    stream,
                    requested: None,
                },
                &mut clients,
                b"sample\n",
            );
        }
        assert_eq!(clients.len(), MAX_CLIENTS);
    }
}
