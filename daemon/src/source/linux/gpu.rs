//! Linux NVIDIA GPU load, memory and temperature through NVML.
//!
//! NVML is loaded once at startup: its initialiser resolves every symbol in
//! the driver library, so repeating it per tick would dominate the cost of the
//! whole sample. Absence of a driver is not an error — the daemon simply
//! reports no GPU and every other metric keeps flowing.

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
        // NVML is the only interface that reports NVIDIA load: the proprietary
        // driver publishes nothing but static identity under procfs and binds
        // no hwmon node. It is also the daemon's entire memory cost — roughly
        // twenty megabytes of driver-side state against half a megabyte for
        // everything else — so it stays one environment variable away.
        if std::env::var_os("RLDYOUR_SYSINFO_GPU").is_some_and(|value| value == "0") {
            return Self { nvml: None };
        }
        Self {
            nvml: nvml_wrapper::Nvml::init().ok(),
        }
    }

    pub fn read(&mut self) -> Option<Reading> {
        use nvml_wrapper::enum_wrappers::device::TemperatureSensor;

        let nvml = self.nvml.as_ref()?;
        // Cheap handle lookup against the already-initialised library; the
        // device cannot be cached because it borrows the NVML instance.
        let device = nvml.device_by_index(0).ok()?;

        let usage = device.utilization_rates().ok()?.gpu as f32;
        let memory = device.memory_info().ok()?;
        let memory = if memory.total == 0 {
            0.0
        } else {
            memory.used as f32 * 100.0 / memory.total as f32
        };

        Some(Reading {
            usage,
            memory,
            temperature: device
                .temperature(TemperatureSensor::Gpu)
                .ok()
                .map(|t| t as f32),
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

    pub fn read(&mut self) -> Option<Reading> {
        None
    }
}
