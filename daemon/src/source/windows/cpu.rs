//! Windows aggregate CPU load from system counters.

use sysinfo::{CpuRefreshKind, RefreshKind, System};

pub struct Cpu {
    system: System,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            system: System::new_with_specifics(
                RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
            ),
        }
    }

    /// Busy share of all cores since the previous call, in percent.
    ///
    /// sysinfo primes the counter baseline at construction, so unlike the
    /// counter-diff readers the first call already returns a real figure.
    pub fn usage(&mut self) -> f32 {
        self.system.refresh_cpu_usage();
        self.system.global_cpu_usage()
    }
}
