//! macOS GPU utilisation and temperature from IOAccelerator statistics.

use super::average;
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

    /// Averages across every accelerator the registry reports.
    pub fn read(&self) -> Reading {
        let mut usages = Vec::new();
        let mut temperatures = Vec::new();

        if let Ok(services) = matching_services("IOAccelerator") {
            for service in services {
                let Ok(Some(CFValue::Dictionary(stats))) =
                    service.property("PerformanceStatistics")
                else {
                    continue;
                };
                for key in ["Device Utilization %", "GPU Activity(%)"] {
                    if let Some(CFValue::Integer(value)) = stats.get(key) {
                        usages.push((*value).clamp(0, 100) as f32);
                        break;
                    }
                }
                if let Some(CFValue::Integer(value)) = stats.get("Temperature(C)") {
                    if (1..=120).contains(value) {
                        temperatures.push(*value as f32);
                    }
                }
            }
        }

        Reading {
            usage: average(&usages),
            temperature: average(&temperatures),
        }
    }
}
