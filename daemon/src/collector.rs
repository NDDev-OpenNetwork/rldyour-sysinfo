//! Assembles one snapshot per tick from the individual sources.
//!
//! Sources that fail to initialise are dropped rather than fatal: a machine
//! without an NVIDIA card or without a temperature driver still gets every
//! other metric. A source that fails mid-run reports `None` for that tick and
//! is retried on the next one.

use crate::proto::Snapshot;
use crate::source::{cpu::Cpu, disk::Disk, gpu::Gpu, mem::Memory, net::Network, temp::Sensor};
use std::io;
use std::time::Instant;

pub struct Collector {
    cpu: Cpu,
    memory: Memory,
    disk: Disk,
    network: Network,
    gpu: Gpu,
    cpu_temperature: Option<Sensor>,
    disk_temperature: Option<Sensor>,
    sampled_at: Instant,
}

impl Collector {
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

    /// Reads every source once. Rates are derived from the real elapsed time
    /// rather than the nominal interval, so a late tick does not inflate them.
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
