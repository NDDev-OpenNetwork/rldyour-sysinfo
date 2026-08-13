//! Aggregate CPU busy time from `/proc/stat`.

use super::{VirtualFile, delta};
use std::io;

pub struct Cpu {
    file: VirtualFile,
    previous: Option<Sample>,
}

#[derive(Clone, Copy)]
struct Sample {
    total: u64,
    idle: u64,
}

impl Cpu {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            file: VirtualFile::open("/proc/stat")?,
            previous: None,
        })
    }

    /// Busy share of all cores since the previous call, in percent.
    ///
    /// The first call establishes the baseline and reports nothing, because a
    /// percentage needs two samples.
    pub fn usage(&mut self) -> io::Result<Option<f32>> {
        let line = self.file.read()?.lines().next().unwrap_or_default();

        // `cpu  user nice system idle iowait irq softirq steal guest guest_nice`
        let mut total = 0u64;
        let mut idle = 0u64;
        for (position, value) in line.split_ascii_whitespace().skip(1).enumerate() {
            let Ok(ticks) = value.parse::<u64>() else {
                continue;
            };
            total += ticks;
            // `idle` and `iowait` both mean the core had nothing to run.
            if position == 3 || position == 4 {
                idle += ticks;
            }
        }

        let current = Sample { total, idle };
        let previous = self.previous.replace(current);

        Ok(previous.and_then(|previous| {
            let elapsed = delta(current.total, previous.total);
            if elapsed == 0 {
                return None;
            }
            let busy = elapsed - delta(current.idle, previous.idle).min(elapsed);
            Some(busy as f32 * 100.0 / elapsed as f32)
        }))
    }
}
