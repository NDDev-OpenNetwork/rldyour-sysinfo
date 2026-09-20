//! macOS storage throughput from IOKit block storage statistics.

use crate::source::{delta, rate};
use iokit::{CFValue, matching_services};

pub struct Disk {
    previous: Option<Sample>,
}

#[derive(Clone, Copy)]
struct Sample {
    read: u64,
    write: u64,
}

pub struct Throughput {
    pub read: u64,
    pub write: u64,
}

impl Disk {
    pub fn new() -> Self {
        Self { previous: None }
    }

    /// Bytes read and written per second across all storage drivers.
    pub fn throughput(&mut self, seconds: f64) -> Option<Throughput> {
        let current = read_counters()?;
        let previous = self.previous.replace(current);

        previous.map(|previous| Throughput {
            read: rate(delta(current.read, previous.read), seconds),
            write: rate(delta(current.write, previous.write), seconds),
        })
    }
}

/// Cumulative byte counters summed across every `IOBlockStorageDriver`.
fn read_counters() -> Option<Sample> {
    let services = matching_services("IOBlockStorageDriver").ok()?;
    let mut sample = Sample { read: 0, write: 0 };
    let mut found = false;
    for service in services {
        let Ok(Some(CFValue::Dictionary(stats))) = service.property("Statistics") else {
            continue;
        };
        if let Some(CFValue::Integer(value)) = stats.get("Bytes (Read)") {
            sample.read = sample.read.saturating_add((*value).max(0) as u64);
            found = true;
        }
        if let Some(CFValue::Integer(value)) = stats.get("Bytes (Write)") {
            sample.write = sample.write.saturating_add((*value).max(0) as u64);
            found = true;
        }
    }
    found.then_some(sample)
}
