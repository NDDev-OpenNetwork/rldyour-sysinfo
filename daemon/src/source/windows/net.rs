//! Windows physical interface throughput from network counters.
//!
//! sysinfo does not expose an interface's media type, so "physical" is
//! approximated by excluding the names Windows gives its software adapters:
//! loopback, pseudo-interfaces, tunnels, and the virtual switches that
//! hypervisors and VPN clients create.

use crate::source::{delta, rate};
use sysinfo::Networks;

/// Substrings that mark an adapter as software-emulated rather than a wire.
const VIRTUAL_ADAPTERS: &[&str] = &[
    "loopback",
    "pseudo",
    "teredo",
    "isatap",
    "6to4",
    "tunnel",
    "bluetooth",
    "vethernet",
    "hyper-v",
    "wsl",
    "virtualbox",
    "vmware",
    "tap-",
    "tailscale",
    "wireguard",
    "zerotier",
    "docker",
];

pub struct Network {
    networks: Networks,
    previous: Option<Sample>,
}

#[derive(Clone, Copy)]
struct Sample {
    received: u64,
    transmitted: u64,
}

pub struct Throughput {
    pub rx: u64,
    pub tx: u64,
}

impl Network {
    pub fn new() -> Self {
        Self {
            networks: Networks::new_with_refreshed_list(),
            previous: None,
        }
    }

    /// Bytes received and transmitted per second across physical interfaces.
    ///
    /// The first call establishes the baseline and reports nothing, because a
    /// rate needs two samples.
    pub fn throughput(&mut self, seconds: f64) -> Option<Throughput> {
        self.networks.refresh(true);
        let mut received = 0u64;
        let mut transmitted = 0u64;
        let mut found = false;
        for (name, data) in self.networks.iter() {
            if VIRTUAL_ADAPTERS
                .iter()
                .any(|marker| contains_marker(name, marker))
            {
                continue;
            }
            received = received.saturating_add(data.total_received());
            transmitted = transmitted.saturating_add(data.total_transmitted());
            found = true;
        }
        if !found {
            return None;
        }
        let previous = self.previous.replace(Sample {
            received,
            transmitted,
        });
        previous.map(|previous| Throughput {
            rx: rate(delta(received, previous.received), seconds),
            tx: rate(delta(transmitted, previous.transmitted), seconds),
        })
    }
}

/// ASCII-insensitive substring test, so classifying an adapter costs no
/// lowercase copy on every tick. The markers are all ASCII, which is all this
/// needs to fold.
fn contains_marker(name: &str, marker: &str) -> bool {
    name.as_bytes()
        .windows(marker.len())
        .any(|window| window.eq_ignore_ascii_case(marker.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_names_match_case_insensitively() {
        assert!(contains_marker("Tailscale Tunnel", "tailscale"));
        assert!(contains_marker(
            "Hyper-V Virtual Ethernet Adapter",
            "hyper-v"
        ));
        assert!(!contains_marker("Intel Ethernet Controller", "vmware"));
        assert!(!contains_marker("Wi-Fi", "wsl"));
    }
}
