//! rldyour-sysinfod — publishes a system snapshot to connected clients.
//!
//! One reusable JSON buffer and one hardware sampling clock serve every local
//! client. The server bounds admission and I/O; platform collectors own their
//! native resources. No async runtime or process-list scan is required.

// Release builds are background daemons: no console window should ever
// appear when Windows launches the binary from the Run key.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod collector;
mod proto;
mod source;

mod server;
mod transport;

use std::process::ExitCode;

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

    match transport::bind().and_then(|(listener, activated)| server::serve(listener, activated)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rldyour-sysinfod: {error}");
            ExitCode::FAILURE
        }
    }
}
