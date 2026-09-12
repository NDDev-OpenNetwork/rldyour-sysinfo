//! Windows collector using native system APIs through sysinfo and optional NVML.

use crate::proto::Snapshot;
use std::io;
use sysinfo::{Components, CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};

pub struct WindowsCollector {
    system: System,
    networks: Networks,
    components: Components,
    gpu: Gpu,
}

impl WindowsCollector {
    pub fn new() -> io::Result<Self> {
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        Ok(Self {
            system,
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            gpu: Gpu::new(),
        })
    }

    pub fn sample(&mut self) -> Snapshot {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.networks.refresh(true);
        self.components.refresh(false);

        let total_memory = self.system.total_memory();
        let memory = (total_memory > 0)
            .then(|| self.system.used_memory() as f32 * 100.0 / total_memory as f32);
        let total_swap = self.system.total_swap();
        let swap =
            (total_swap > 0).then(|| self.system.used_swap() as f32 * 100.0 / total_swap as f32);
        let net_rx = Some(self.networks.values().map(|data| data.received()).sum());
        let net_tx = Some(self.networks.values().map(|data| data.transmitted()).sum());
        let cpu_temperature = self
            .components
            .iter()
            .filter_map(|component| component.temperature())
            .filter(|temperature| temperature.is_finite() && (0.0..=120.0).contains(temperature))
            .max_by(f32::total_cmp);
        let gpu = self.gpu.read();

        Snapshot {
            cpu: Some(self.system.global_cpu_usage()),
            cpu_temperature,
            memory,
            swap,
            gpu: gpu.as_ref().map(|reading| reading.usage),
            gpu_memory: gpu.as_ref().map(|reading| reading.memory),
            gpu_temperature: gpu.and_then(|reading| reading.temperature),
            net_rx,
            net_tx,
            ..Snapshot::default()
        }
    }
}

struct GpuReading {
    usage: f32,
    memory: f32,
    temperature: Option<f32>,
}

#[cfg(feature = "nvidia")]
struct Gpu(Option<nvml_wrapper::Nvml>);

#[cfg(feature = "nvidia")]
impl Gpu {
    fn new() -> Self {
        Self(nvml_wrapper::Nvml::init().ok())
    }

    fn read(&self) -> Option<GpuReading> {
        use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
        let device = self.0.as_ref()?.device_by_index(0).ok()?;
        let usage = device.utilization_rates().ok()?.gpu as f32;
        let memory = device.memory_info().ok()?;
        Some(GpuReading {
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
struct Gpu;

#[cfg(not(feature = "nvidia"))]
impl Gpu {
    fn new() -> Self {
        Self
    }
    fn read(&self) -> Option<GpuReading> {
        None
    }
}
