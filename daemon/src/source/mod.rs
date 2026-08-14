//! Zero-allocation readers for the kernel's virtual filesystems.
//!
//! Every source keeps its file descriptors open for the process lifetime and
//! rewinds them instead of reopening. `procfs` and `sysfs` regenerate their
//! contents on each read after a seek, so a steady-state tick performs no
//! `open`, and no allocation once the string buffers have grown once.

pub mod cpu;
pub mod disk;
pub mod gpu;
pub mod mem;
pub mod net;
pub mod temp;

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// A kernel virtual file held open and rewound on every read.
pub struct VirtualFile {
    file: File,
    buf: String,
}

impl VirtualFile {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            file: File::open(path)?,
            buf: String::with_capacity(4096),
        })
    }

    /// Rewinds and re-reads the file into the reusable buffer.
    pub fn read(&mut self) -> io::Result<&str> {
        self.file.seek(SeekFrom::Start(0))?;
        self.buf.clear();
        self.file.read_to_string(&mut self.buf)?;
        Ok(&self.buf)
    }
}

/// Returns the numeric field at `index` (0-based) of a whitespace-split line.
pub fn field<T: std::str::FromStr>(line: &str, index: usize) -> Option<T> {
    line.split_ascii_whitespace().nth(index)?.parse().ok()
}

/// Difference between two monotonic counters, tolerating kernel counter resets.
pub fn delta(current: u64, previous: u64) -> u64 {
    current.saturating_sub(previous)
}

/// Converts a byte delta observed over `seconds` into a per-second rate.
pub fn rate(bytes: u64, seconds: f64) -> u64 {
    if seconds <= 0.0 {
        0
    } else {
        (bytes as f64 / seconds) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_positional_field() {
        let line = " 259 0 nvme0n1 2397597 508478 60721278 220367";
        assert_eq!(field::<u64>(line, 2), None::<u64>); // the name is not numeric
        assert_eq!(field::<u64>(line, 5), Some(60721278));
        assert_eq!(field::<u64>(line, 99), None::<u64>);
    }

    #[test]
    fn a_counter_reset_yields_zero_rather_than_wrapping() {
        assert_eq!(delta(10, 4), 6);
        // The kernel can reset a counter; a huge bogus delta would render as a
        // multi-gigabyte spike in the panel.
        assert_eq!(delta(4, 10), 0);
    }

    #[test]
    fn rates_are_per_second_and_reject_a_zero_interval() {
        assert_eq!(rate(1024, 2.0), 512);
        assert_eq!(rate(1024, 0.0), 0);
        assert_eq!(rate(1024, -1.0), 0);
    }
}
