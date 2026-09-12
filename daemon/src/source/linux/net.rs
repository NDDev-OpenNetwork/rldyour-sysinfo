//! Linux physical interface throughput from `/proc/net/dev`.

use super::{VirtualFile, delta, field, rate};
use std::io;

pub struct Network {
    file: VirtualFile,
    /// Physical interfaces only, so container bridges and veth pairs do not
    /// count the same packet twice as it is forwarded.
    interfaces: Vec<String>,
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
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            file: VirtualFile::open("/proc/net/dev")?,
            interfaces: physical_interfaces()?,
            previous: None,
        })
    }

    /// Bytes received and transmitted per second across physical interfaces.
    pub fn throughput(&mut self, seconds: f64) -> io::Result<Option<Throughput>> {
        let (mut received, mut transmitted) = (0u64, 0u64);

        for line in self.file.read()?.lines() {
            let Some((name, counters)) = line.split_once(':') else {
                continue;
            };
            let name = name.trim();
            if !self.interfaces.iter().any(|interface| interface == name) {
                continue;
            }
            received += field::<u64>(counters, 0).unwrap_or(0);
            transmitted += field::<u64>(counters, 8).unwrap_or(0);
        }

        let current = Sample {
            received,
            transmitted,
        };
        let previous = self.previous.replace(current);

        Ok(previous.map(|previous| Throughput {
            rx: rate(delta(current.received, previous.received), seconds),
            tx: rate(delta(current.transmitted, previous.transmitted), seconds),
        }))
    }
}

/// Interfaces with a backing device, which excludes loopback, bridges and veth.
fn physical_interfaces() -> io::Result<Vec<String>> {
    let mut interfaces = Vec::new();
    for entry in std::fs::read_dir("/sys/class/net")? {
        let entry = entry?;
        if entry.path().join("device").exists() {
            interfaces.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    Ok(interfaces)
}
