//! Windows storage throughput from per-volume performance counters.

use crate::source::{delta, rate};
use sysinfo::Disks;

pub struct Disk {
    disks: Disks,
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
        Self {
            disks: Disks::new_with_refreshed_list(),
            previous: None,
        }
    }

    /// Bytes read and written per second summed across every volume.
    ///
    /// The first call establishes the baseline and reports nothing, because a
    /// rate needs two samples.
    pub fn throughput(&mut self, seconds: f64) -> Option<Throughput> {
        self.disks.refresh(true);
        if self.disks.is_empty() {
            return None;
        }
        let current = self.disks.iter().map(|disk| disk.usage()).fold(
            Sample { read: 0, write: 0 },
            |sum, usage| Sample {
                read: sum.read.saturating_add(usage.total_read_bytes),
                write: sum.write.saturating_add(usage.total_written_bytes),
            },
        );
        let previous = self.previous.replace(current);
        previous.map(|previous| Throughput {
            read: rate(delta(current.read, previous.read), seconds),
            write: rate(delta(current.write, previous.write), seconds),
        })
    }
}
