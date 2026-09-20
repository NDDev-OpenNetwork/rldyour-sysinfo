//! Linux collector backed directly by procfs, sysfs, and optional NVML.

mod cpu;
mod disk;
mod mem;
mod net;
mod temp;

use crate::proto::Snapshot;
use crate::source::nvidia::Gpu;
use cpu::Cpu;
use disk::Disk;
use mem::Memory;
use net::Network;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Instant;
use temp::Sensor;

pub struct LinuxCollector {
    cpu: Cpu,
    memory: Memory,
    disk: Disk,
    network: Network,
    gpu: Gpu,
    cpu_temperature: Option<Sensor>,
    disk_temperature: Option<Sensor>,
    sampled_at: Instant,
}

impl LinuxCollector {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            cpu: Cpu::new()?,
            memory: Memory::new()?,
            disk: Disk::new()?,
            network: Network::new()?,
            gpu: Gpu::new(),
            cpu_temperature: Sensor::cpu(),
            disk_temperature: Sensor::disk(),
            sampled_at: Instant::now(),
        })
    }

    pub fn sample(&mut self) -> Snapshot {
        let now = Instant::now();
        let seconds = now.duration_since(self.sampled_at).as_secs_f64();
        self.sampled_at = now;
        let mut snapshot = Snapshot {
            cpu: self.cpu.usage().ok().flatten(),
            ..Snapshot::default()
        };
        if let Ok(Some(usage)) = self.memory.usage() {
            snapshot.memory = Some(usage.used);
            snapshot.swap = usage.swap;
        }
        if let Ok(Some(throughput)) = self.disk.throughput(seconds) {
            snapshot.disk_read = Some(throughput.read);
            snapshot.disk_write = Some(throughput.write);
        }
        if let Ok(Some(throughput)) = self.network.throughput(seconds) {
            snapshot.net_rx = Some(throughput.rx);
            snapshot.net_tx = Some(throughput.tx);
        }
        if let Some(reading) = self.gpu.read() {
            snapshot.gpu = Some(reading.usage);
            snapshot.gpu_memory = Some(reading.memory);
            snapshot.gpu_temperature = reading.temperature;
        }
        snapshot.cpu_temperature = read_sensor(self.cpu_temperature.as_mut());
        snapshot.disk_temperature = read_sensor(self.disk_temperature.as_mut());
        snapshot
    }
}

fn read_sensor(sensor: Option<&mut Sensor>) -> Option<f32> {
    sensor?.celsius().ok().flatten()
}

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

    pub fn read(&mut self) -> io::Result<&str> {
        self.file.seek(SeekFrom::Start(0))?;
        self.buf.clear();
        self.file.read_to_string(&mut self.buf)?;
        Ok(&self.buf)
    }
}

pub fn field<T: std::str::FromStr>(line: &str, index: usize) -> Option<T> {
    line.split_ascii_whitespace().nth(index)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_reads_the_nth_column() {
        let line = " 259 0 nvme0n1 2397597 508478 60721278 220367";
        assert_eq!(field::<u64>(line, 5), Some(60721278));
    }
}
