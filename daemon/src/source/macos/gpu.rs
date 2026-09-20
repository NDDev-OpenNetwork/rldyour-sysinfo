//! macOS GPU utilisation and temperature from IOAccelerator statistics.

use super::mean;
use iokit::{CFValue, matching_services};

/// The accelerator's own report. Either field may be absent: virtual machines
/// expose no `IOAccelerator` service at all, and not every device publishes a
/// temperature.
pub struct Reading {
    /// Share of the sampling period the GPU was busy, in percent.
    pub usage: Option<f32>,
    pub temperature: Option<f32>,
}

pub struct Gpu;

impl Gpu {
    pub fn new() -> Self {
        Self
    }

    /// Averages across every accelerator the registry reports. Readings are
    /// accumulated as (sum, count) pairs rather than collected, so a tick
    /// allocates nothing beyond what IOKit itself needs.
    pub fn read(&self) -> Reading {
        let mut usages = (0.0, 0);
        let mut temperatures = (0.0, 0);

        if let Ok(services) = matching_services("IOAccelerator") {
            for service in services {
                let Ok(Some(CFValue::Dictionary(stats))) =
                    service.property("PerformanceStatistics")
                else {
                    continue;
                };
                for key in ["Device Utilization %", "GPU Activity(%)"] {
                    if let Some(CFValue::Integer(value)) = stats.get(key) {
                        usages.0 += (*value).clamp(0, 100) as f64;
                        usages.1 += 1;
                        break;
                    }
                }
                if let Some(CFValue::Integer(value)) = stats.get("Temperature(C)") {
                    if (1..=120).contains(value) {
                        temperatures.0 += *value as f64;
                        temperatures.1 += 1;
                    }
                }
            }
        }

        Reading {
            usage: mean(usages.0, usages.1),
            temperature: mean(temperatures.0, temperatures.1),
        }
    }
}
