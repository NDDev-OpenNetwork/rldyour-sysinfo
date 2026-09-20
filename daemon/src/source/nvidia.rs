//! NVIDIA GPU load, memory and temperature through NVML.
//!
//! The proprietary NVIDIA driver publishes no utilisation or temperature
//! through procfs, sysfs or an hwmon node, and exposes nothing comparable on
//! Windows either, so NVML is the only interface that answers on both
//! platforms. It is loaded once at startup: its initialiser resolves every
//! symbol in the driver library, so repeating it per tick would dominate the
//! cost of the whole sample.
//!
//! NVML is also the daemon's entire memory cost — roughly twenty megabytes of
//! driver-side state against half a megabyte for everything else — which is
//! why it stays one environment variable away and can be compiled out with
//! `--no-default-features`. Absence of a driver is not an error: the daemon
//! simply reports no GPU and every other metric keeps flowing.

/// One reading off the primary adapter.
pub struct Reading {
    /// Share of the sampling period the GPU was busy, in percent.
    pub usage: f32,
    /// Share of video memory in use, in percent.
    pub memory: f32,
    pub temperature: Option<f32>,
}

#[cfg(feature = "nvidia")]
pub struct Gpu {
    nvml: Option<nvml_wrapper::Nvml>,
}

#[cfg(feature = "nvidia")]
impl Gpu {
    pub fn new() -> Self {
        if std::env::var_os("RLDYOUR_SYSINFO_GPU").is_some_and(|value| value == "0") {
            return Self { nvml: None };
        }
        Self {
            nvml: nvml_wrapper::Nvml::init().ok(),
        }
    }

    pub fn read(&self) -> Option<Reading> {
        use nvml_wrapper::enum_wrappers::device::TemperatureSensor;

        // Cheap handle lookup against the already-initialised library; the
        // device cannot be cached because it borrows the NVML instance.
        let device = self.nvml.as_ref()?.device_by_index(0).ok()?;
        let usage = device.utilization_rates().ok()?.gpu as f32;
        let memory = device.memory_info().ok()?;

        Some(Reading {
            usage,
            memory: if memory.total == 0 {
                0.0
            } else {
                memory.used as f32 * 100.0 / memory.total as f32
            },
            temperature: device
                .temperature(TemperatureSensor::Gpu)
                .ok()
                .map(|value| value as f32),
        })
    }
}

#[cfg(not(feature = "nvidia"))]
pub struct Gpu;

#[cfg(not(feature = "nvidia"))]
impl Gpu {
    pub fn new() -> Self {
        Self
    }

    pub fn read(&self) -> Option<Reading> {
        None
    }
}
