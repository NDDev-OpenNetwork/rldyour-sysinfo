//! Physical network throughput with hotplug-safe per-device counters.

use super::devices::Devices;
use super::{VirtualFile, field};
use std::io;
use std::time::Instant;

pub struct Network {
    file: VirtualFile,
    devices: Devices,
}
pub struct Throughput {
    pub rx: u64,
    pub tx: u64,
}

impl Network {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            file: VirtualFile::open("/proc/net/dev")?,
            devices: Devices::new("/sys/class/net")?,
        })
    }

    pub fn throughput(&mut self) -> io::Result<Option<Throughput>> {
        self.devices.refresh();
        let now = Instant::now();
        let mut total = Throughput { rx: 0, tx: 0 };
        let mut found = false;
        for line in self.file.read()?.lines() {
            let Some((name, counters)) = line.split_once(':') else {
                continue;
            };
            let name = name.trim();
            let Some(first) = field::<u64>(counters, 0) else {
                continue;
            };
            let Some(second) = field::<u64>(counters, 8) else {
                continue;
            };
            let Some(device) = self
                .devices
                .entries
                .iter_mut()
                .find(|device| device.name == name)
            else {
                continue;
            };
            // diskstats sectors are always 512 bytes, regardless of block size.
            if let Some((first, second)) =
                device
                    .counter
                    .observe(first.saturating_mul(1), second.saturating_mul(1), now)
            {
                total.rx = total.rx.saturating_add(first);
                total.tx = total.tx.saturating_add(second);
                found = true;
            }
        }
        Ok(found.then_some(total))
    }
}
