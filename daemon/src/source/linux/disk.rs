//! Linux whole-disk throughput from `/proc/diskstats`.

use super::{VirtualFile, delta, field, rate};
use std::io;

/// The kernel reports disk transfers in fixed 512-byte units regardless of the
/// device's own logical block size.
const SECTOR_BYTES: u64 = 512;

pub struct Disk {
    file: VirtualFile,
    /// Whole disks only; counting partitions too would double every transfer.
    devices: Vec<String>,
    previous: Option<Sample>,
}

#[derive(Clone, Copy)]
struct Sample {
    read: u64,
    written: u64,
}

pub struct Throughput {
    pub read: u64,
    pub write: u64,
}

impl Disk {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            file: VirtualFile::open("/proc/diskstats")?,
            devices: physical_disks()?,
            previous: None,
        })
    }

    /// Bytes read and written per second across all physical disks.
    pub fn throughput(&mut self, seconds: f64) -> io::Result<Option<Throughput>> {
        let (mut read, mut written) = (0u64, 0u64);

        for line in self.file.read()?.lines() {
            let Some(name) = line.split_ascii_whitespace().nth(2) else {
                continue;
            };
            if !self.devices.iter().any(|device| device == name) {
                continue;
            }
            read += field::<u64>(line, 5).unwrap_or(0);
            written += field::<u64>(line, 9).unwrap_or(0);
        }

        let current = Sample { read, written };
        let previous = self.previous.replace(current);

        Ok(previous.map(|previous| Throughput {
            read: rate(delta(current.read, previous.read) * SECTOR_BYTES, seconds),
            write: rate(
                delta(current.written, previous.written) * SECTOR_BYTES,
                seconds,
            ),
        }))
    }
}

/// Block devices backed by real hardware, excluding loop, ram and zram.
fn physical_disks() -> io::Result<Vec<String>> {
    let mut disks = Vec::new();
    for entry in std::fs::read_dir("/sys/block")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // A `device` link is what separates a real disk from a virtual one.
        if entry.path().join("device").exists() {
            disks.push(name);
        }
    }
    Ok(disks)
}
