//! Windows memory and swap pressure from system counters.

use sysinfo::{MemoryRefreshKind, RefreshKind, System};

pub struct Memory {
    system: System,
}

pub struct Usage {
    /// Share of physical memory in use, in percent.
    pub used: f32,
    /// Share of swap in use, in percent. `None` when the system has no swap.
    pub swap: Option<f32>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            system: System::new_with_specifics(
                RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
            ),
        }
    }

    pub fn usage(&mut self) -> Option<Usage> {
        self.system.refresh_memory();
        let total = self.system.total_memory();
        if total == 0 {
            return None;
        }
        let swap_total = self.system.total_swap();
        Some(Usage {
            used: self.system.used_memory() as f32 * 100.0 / total as f32,
            swap: (swap_total > 0)
                .then(|| self.system.used_swap() as f32 * 100.0 / swap_total as f32),
        })
    }
}
