//! rldyour-sysinfod — publishes a system snapshot to connected clients.
//!
//! The daemon is deliberately single-purpose: one timer, one buffer, one
//! newline-delimited JSON line per tick to every connected client. There is no
//! async runtime and no D-Bus stack, because a 0.2 Hz cadence carrying under
//! two hundred bytes does not pay for either. Steady-state operation performs
//! no allocation: every file descriptor and every buffer is created once.

// Release builds are background daemons: no console window should ever
// appear when Windows launches the binary from the Run key.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod collector;
mod proto;
mod source;

use collector::Collector;
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::ExitCode;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::time::{Duration, Instant};
#[cfg(windows)]
use uds_windows::{UnixListener, UnixStream};

/// Publication cadence when nothing asks for another. Overridable through
/// `RLDYOUR_SYSINFO_INTERVAL`, expressed in whole seconds.
const DEFAULT_INTERVAL: Duration = Duration::from_secs(5);
/// Bounds for a cadence a client may request. Zero is not "never" but the
/// realtime mode, a half-second tick; above a minute the panel stops being one.
const MIN_INTERVAL: u64 = 0;
const MAX_INTERVAL: u64 = 60;
/// The tick realtime mode runs at. Half a second is the floor where kernel
/// counters and sensor reads still cost less than the update is worth — and
/// stays above sysinfo's minimum CPU refresh window on Windows.
const REALTIME_INTERVAL: Duration = Duration::from_millis(500);
/// A client announces itself immediately or not at all, so this wait only ever
/// costs anything for a peer that connected and then went quiet.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_millis(250);
/// A client that cannot absorb one short line in this long is treated as gone,
/// so one stuck reader can never stall publication for the others.
const WRITE_TIMEOUT: Duration = Duration::from_millis(500);
/// When systemd owns the listening socket the daemon may exit once nobody is
/// watching; the next connection starts it again.
const IDLE_EXIT: Duration = Duration::from_secs(30);
/// The accept loop needs almost no stack, and the default eight megabytes of
/// reservation is the single largest line item in the process map.
const ACCEPT_STACK: usize = 64 * 1024;
const SOCKET_NAME: &str = "rldyour-sysinfo.sock";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        None => {}
        Some("--version" | "-V") => {
            println!("rldyour-sysinfod {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some("--help" | "-h") => {
            println!(
                "rldyour-sysinfod {} — system metrics daemon\n\
                 \n\
                 Publishes one newline-terminated JSON sample per interval to every\n\
                 client connected to its local socket. No arguments are needed;\n\
                 the socket location and cadence come from the environment.\n\
                 \n\
                 Environment:\n\
                 \x20 RLDYOUR_SYSINFO_INTERVAL  Cadence in seconds when no client asks\n\
                 \x20                           (default 5; 0 selects half-second realtime)\n\
                 \x20 RLDYOUR_SYSINFO_GPU       Set to 0 to skip NVIDIA metrics entirely",
                env!("CARGO_PKG_VERSION")
            );
            return ExitCode::SUCCESS;
        }
        Some(argument) => {
            eprintln!("rldyour-sysinfod: unknown argument {argument:?}");
            return ExitCode::FAILURE;
        }
    }

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rldyour-sysinfod: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> std::io::Result<()> {
    let (listener, socket_activated) = bind()?;
    let mut collector = Collector::new()?;
    let configured = interval();

    let clients = accept_loop(listener);
    let mut connected: Vec<Client> = Vec::new();
    let mut line = String::with_capacity(256);
    let mut idle_since = Some(Instant::now());

    loop {
        // Wait out the cadence on the accept channel itself: a new connection
        // wakes the loop at once, so its first sample is not a whole tick
        // late. No polling — the channel is the wakeup.
        match clients.recv_timeout(cadence(&connected, configured)) {
            Ok(client) => {
                let _ = client.stream.set_write_timeout(Some(WRITE_TIMEOUT));
                connected.push(client);
                idle_since = None;
            }
            Err(RecvTimeoutError::Timeout) => {}
            // The accept thread only ends if the listener itself died.
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }

        loop {
            match clients.try_recv() {
                Ok(client) => {
                    let _ = client.stream.set_write_timeout(Some(WRITE_TIMEOUT));
                    connected.push(client);
                    idle_since = None;
                }
                Err(TryRecvError::Empty) => break,
                // The accept thread only ends if the listener itself died.
                Err(TryRecvError::Disconnected) => return Ok(()),
            }
        }

        // Sampling continues without clients so that the counter deltas stay
        // warm and the first line after a reconnect is already meaningful.
        collector.sample().encode(&mut line);

        let bytes = line.as_bytes();
        connected.retain_mut(|client| client.stream.write_all(bytes).is_ok());

        match (connected.is_empty(), idle_since) {
            (false, _) => idle_since = None,
            (true, None) => idle_since = Some(Instant::now()),
            (true, Some(since)) => {
                if socket_activated && since.elapsed() >= IDLE_EXIT {
                    return Ok(());
                }
            }
        }
    }
}

/// Uses the socket the service manager passed in, or binds one under the
/// platform's per-user directory.
///
/// Returns the listener and whether the manager owns it, which decides if the
/// daemon is allowed to exit when idle.
fn bind() -> std::io::Result<(UnixListener, bool)> {
    if let Some(listener) = inherited_listener() {
        return Ok((listener, true));
    }

    let path = socket_path()?;

    // A socket file left behind by an unclean exit would otherwise make the
    // bind fail with EADDRINUSE even though nothing is listening.
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;

    // systemd already delivers the socket at 0600; a self-bound one gets the
    // same restriction explicitly rather than whatever the umask happens to be.
    // Only the owner may connect to a socket that reports this host's metrics.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok((listener, false))
}

/// The socket location follows each platform's own convention, and only that
/// convention: honouring `XDG_RUNTIME_DIR` outside Linux would strand the
/// clients, which look in the platform directory and nowhere else.
fn socket_path() -> std::io::Result<std::path::PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| {
            std::io::Error::other("XDG_RUNTIME_DIR is unset and no socket was inherited")
        })?;
        return Ok(std::path::Path::new(&runtime).join(SOCKET_NAME));
    }

    #[cfg(target_os = "macos")]
    {
        let home =
            std::env::var_os("HOME").ok_or_else(|| std::io::Error::other("HOME is unset"))?;
        // Library/Caches is purgeable under disk pressure, which would strand
        // the socket of a live daemon; Application Support is not.
        let directory =
            std::path::Path::new(&home).join("Library/Application Support/rldyour-sysinfo");
        std::fs::create_dir_all(&directory)?;
        return Ok(directory.join(SOCKET_NAME));
    }

    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("TEMP"))
            .ok_or_else(|| std::io::Error::other("LOCALAPPDATA and TEMP are unset"))?;
        let directory = std::path::Path::new(&base).join("rldyour-sysinfo");
        std::fs::create_dir_all(&directory)?;
        return Ok(directory.join(SOCKET_NAME));
    }

    #[allow(unreachable_code)]
    Err(std::io::Error::other("no socket location on this platform"))
}

/// The listening socket the platform service manager passed in, if any.
///
/// systemd hands its socket over as descriptor 3 when LISTEN_FDS/LISTEN_PID
/// name this process; launchd keeps it in the `Sockets` dictionary and hands
/// it out through `launch_activate_socket`. Both let the daemon exit when
/// idle because the next connection relaunches it.
#[cfg(target_os = "linux")]
fn inherited_listener() -> Option<UnixListener> {
    let listening = std::env::var("LISTEN_FDS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok());
    let owner = std::env::var("LISTEN_PID")
        .ok()
        .and_then(|v| v.parse::<u32>().ok());
    if !(listening.is_some_and(|count| count >= 1) && owner == Some(std::process::id())) {
        return None;
    }
    use std::os::fd::FromRawFd;
    // SAFETY: systemd guarantees descriptor 3 is the listening socket it
    // created for this unit, and it is passed to exactly one process.
    Some(unsafe { UnixListener::from_raw_fd(3) })
}

#[cfg(target_os = "macos")]
fn inherited_listener() -> Option<UnixListener> {
    use std::ffi::CString;
    use std::os::fd::FromRawFd;
    use std::os::raw::{c_char, c_int};

    unsafe extern "C" {
        /// In libSystem since launchd exists; returns an error code when this
        /// process was not launched with a `Sockets` dictionary.
        fn launch_activate_socket(
            name: *const c_char,
            fds: *mut *mut c_int,
            cnt: *mut usize,
        ) -> c_int;
    }

    // The key must match the plist's `Sockets` entry.
    let name = CString::new("sock").ok()?;
    let mut fds: *mut c_int = std::ptr::null_mut();
    let mut count: usize = 0;
    if unsafe { launch_activate_socket(name.as_ptr(), &mut fds, &mut count) } != 0
        || fds.is_null()
        || count == 0
    {
        return None;
    }
    // The plist declares exactly one socket; the array is launchd-owned and
    // the caller frees it.
    let fd = unsafe { *fds };
    unsafe { libc::free(fds.cast()) };
    // SAFETY: launch_activate_socket returned this descriptor to us; it is a
    // listening socket nobody else owns.
    Some(unsafe { UnixListener::from_raw_fd(fd) })
}

#[cfg(target_os = "windows")]
fn inherited_listener() -> Option<UnixListener> {
    // Windows has no per-user service manager with socket activation; the
    // daemon always binds its own and stays resident.
    None
}

fn interval() -> Duration {
    std::env::var("RLDYOUR_SYSINFO_INTERVAL")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|seconds| *seconds <= MAX_INTERVAL)
        .map_or(DEFAULT_INTERVAL, |seconds| match seconds {
            0 => REALTIME_INTERVAL,
            _ => Duration::from_secs(seconds),
        })
}

/// A connected client together with the cadence it asked for.
struct Client {
    stream: UnixStream,
    requested: Option<u64>,
}

/// The fastest cadence anybody asked for, falling back to the configured one.
///
/// Taking the minimum means one client wanting a brisk panel does not force a
/// slow one to keep up, and an unannounced client never slows anyone down.
fn cadence(clients: &[Client], configured: Duration) -> Duration {
    clients
        .iter()
        .filter_map(|client| client.requested)
        .min()
        .map_or(configured, |seconds| match seconds {
            // Zero is the realtime request, not "never publish".
            0 => REALTIME_INTERVAL,
            _ => Duration::from_secs(seconds),
        })
}

/// Reads the optional opening line in which a client states its wanted cadence.
fn handshake(stream: &UnixStream) -> Option<u64> {
    stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT)).ok()?;

    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).ok()?;

    parse_interval(&line)
}

/// Extracts a cadence request from `{"interval":N}`.
///
/// `0` asks for realtime; anything else, including silence, yields `None` and
/// leaves the daemon on its configured cadence, so a client that says nothing
/// still works.
fn parse_interval(line: &str) -> Option<u64> {
    let seconds: u64 = line
        .split_once("\"interval\"")?
        .1
        .trim_start()
        .strip_prefix(':')?
        .trim_start()
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;

    (MIN_INTERVAL..=MAX_INTERVAL)
        .contains(&seconds)
        .then_some(seconds)
}

/// Moves blocking accepts off the timing thread, so a connection arriving mid
/// interval never delays the next sample.
fn accept_loop(listener: UnixListener) -> Receiver<Client> {
    let (sender, receiver) = mpsc::channel();

    let spawned = std::thread::Builder::new()
        .name("accept".into())
        .stack_size(ACCEPT_STACK)
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                // The handshake blocks briefly, which is exactly why accepting
                // lives off the timing thread.
                let requested = handshake(&stream);
                if sender.send(Client { stream, requested }).is_err() {
                    return;
                }
            }
        });

    // A daemon that cannot accept connections has nothing to do; failing to
    // spawn is unrecoverable rather than degraded.
    if spawned.is_err() {
        eprintln!("rldyour-sysinfod: cannot start the accept thread");
        std::process::exit(1);
    }

    receiver
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_cadence_request() {
        assert_eq!(parse_interval(r#"{"interval":3}"#), Some(3));
        assert_eq!(parse_interval(r#"{ "interval" : 12 }"#), Some(12));
        // Zero is a real request: the realtime mode.
        assert_eq!(parse_interval(r#"{"interval":0}"#), Some(0));
    }

    #[test]
    fn rejects_a_cadence_outside_what_a_panel_can_use() {
        assert_eq!(parse_interval(r#"{"interval":61}"#), None);
        assert_eq!(parse_interval(r#"{"interval":-1}"#), None);
    }

    #[test]
    fn silence_and_noise_leave_the_configured_cadence_alone() {
        assert_eq!(parse_interval(""), None);
        assert_eq!(parse_interval("hello\n"), None);
        assert_eq!(parse_interval(r#"{"interval":"fast"}"#), None);
    }

    #[test]
    fn the_fastest_request_wins_and_silence_does_not_slow_anyone() {
        let configured = Duration::from_secs(5);
        assert_eq!(cadence(&[], configured), configured);
        assert_eq!(
            cadence(&[client(Some(0)), client(Some(2))], configured),
            REALTIME_INTERVAL
        );
        assert_eq!(
            cadence(&[client(Some(10)), client(None)], configured),
            Duration::from_secs(10)
        );
    }

    fn client(requested: Option<u64>) -> Client {
        Client {
            stream: UnixStream::pair().unwrap().0,
            requested,
        }
    }
}
