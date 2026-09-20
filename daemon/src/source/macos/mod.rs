//! macOS metrics from Mach, BSD, IOKit, and AppleSMC.

use crate::proto::Snapshot;
use crate::source::{delta, rate};
use four_char_code::FourCharCode;
use iokit::{CFValue, matching_services};
use libc::{c_char, c_int, c_uint, c_void};
use std::ffi::CStr;
use std::io;
use std::mem;
use std::ptr;
use std::time::Instant;

const HOST_CPU_LOAD_INFO: c_int = 3;
const HOST_VM_INFO64: c_int = 4;
const CPU_STATE_MAX: usize = 4;
const CPU_STATE_IDLE: usize = 2;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct HostCpuLoadInfo {
    ticks: [u32; CPU_STATE_MAX],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VmStatistics64 {
    free_count: u32,
    active_count: u32,
    inactive_count: u32,
    wire_count: u32,
    zero_fill_count: u64,
    reactivations: u64,
    pageins: u64,
    pageouts: u64,
    faults: u64,
    cow_faults: u64,
    lookups: u64,
    hits: u64,
    purges: u64,
    purgeable_count: u32,
    speculative_count: u32,
    decompressions: u64,
    compressions: u64,
    swapins: u64,
    swapouts: u64,
    compressor_page_count: u32,
    throttled_count: u32,
    external_page_count: u32,
    internal_page_count: u32,
    total_uncompressed_pages_in_compressor: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SwapUsage {
    total: u64,
    available: u64,
    used: u64,
    page_size: u32,
    encrypted: i32,
}

unsafe extern "C" {
    fn mach_host_self() -> c_uint;
    fn host_statistics(host: c_uint, flavor: c_int, info: *mut c_int, count: *mut c_uint) -> c_int;
    fn host_statistics64(
        host: c_uint,
        flavor: c_int,
        info: *mut c_int,
        count: *mut c_uint,
    ) -> c_int;
    fn rldyour_gpu_hid_temperature() -> f64;
}

#[derive(Clone, Copy)]
struct CpuSample {
    total: u64,
    idle: u64,
}

#[derive(Clone, Copy)]
struct NetSample {
    rx: u64,
    tx: u64,
}

#[derive(Clone, Copy)]
struct DiskSample {
    read: u64,
    write: u64,
}

pub struct MacosCollector {
    cpu: Option<CpuSample>,
    network: Option<NetSample>,
    disk: Option<DiskSample>,
    memory_bytes: u64,
    smc: Option<smc::SMC>,
    cpu_temperature_keys: Vec<FourCharCode>,
    gpu_temperature_keys: Vec<FourCharCode>,
    disk_temperature_keys: Vec<FourCharCode>,
    sampled_at: Instant,
}

impl MacosCollector {
    pub fn new() -> io::Result<Self> {
        let memory_bytes = sysctl_u64("hw.memsize")?;
        let smc = smc::SMC::new().ok();
        let (cpu_temperature_keys, gpu_temperature_keys, disk_temperature_keys) = smc
            .as_ref()
            .and_then(|smc| smc.keys().ok())
            .map(classify_temperature_keys)
            .unwrap_or_default();
        Ok(Self {
            cpu: None,
            network: None,
            disk: None,
            memory_bytes,
            smc,
            cpu_temperature_keys,
            gpu_temperature_keys,
            disk_temperature_keys,
            sampled_at: Instant::now(),
        })
    }

    pub fn sample(&mut self) -> Snapshot {
        let now = Instant::now();
        let seconds = now.duration_since(self.sampled_at).as_secs_f64();
        self.sampled_at = now;
        let cpu = cpu_sample().ok().and_then(|current| {
            let previous = self.cpu.replace(current)?;
            let total = current.total.saturating_sub(previous.total);
            let idle = current.idle.saturating_sub(previous.idle).min(total);
            (total > 0).then(|| (total - idle) as f32 * 100.0 / total as f32)
        });

        let memory = memory_used(self.memory_bytes).ok();
        let swap = swap_used().ok().flatten();
        let (net_rx, net_tx) = network_sample()
            .ok()
            .and_then(|current| {
                let previous = self.network.replace(current)?;
                Some((
                    rate(delta(current.rx, previous.rx), seconds),
                    rate(delta(current.tx, previous.tx), seconds),
                ))
            })
            .map_or((None, None), |(rx, tx)| (Some(rx), Some(tx)));

        let (disk_read, disk_write) = disk_sample()
            .and_then(|current| {
                let previous = self.disk.replace(current)?;
                Some((
                    rate(delta(current.read, previous.read), seconds),
                    rate(delta(current.write, previous.write), seconds),
                ))
            })
            .map_or((None, None), |(read, write)| (Some(read), Some(write)));
        let (gpu, accelerator_temperature) = gpu_reading();
        let cpu_temperature = average_temperature(self.smc.as_ref(), &self.cpu_temperature_keys);
        let gpu_temperature = average_temperature(self.smc.as_ref(), &self.gpu_temperature_keys)
            .or(accelerator_temperature)
            .or_else(gpu_hid_temperature);
        let disk_temperature = average_temperature(self.smc.as_ref(), &self.disk_temperature_keys);

        Snapshot {
            cpu,
            cpu_temperature,
            memory,
            swap,
            gpu,
            gpu_temperature,
            disk_read,
            disk_write,
            disk_temperature,
            net_rx,
            net_tx,
            ..Snapshot::default()
        }
    }
}

fn gpu_hid_temperature() -> Option<f32> {
    let value = unsafe { rldyour_gpu_hid_temperature() };
    (value.is_finite() && (10.0..=120.0).contains(&value)).then_some(value as f32)
}

fn gpu_reading() -> (Option<f32>, Option<f32>) {
    let Ok(services) = matching_services("IOAccelerator") else {
        return (None, None);
    };
    let mut usages = Vec::new();
    let mut temperatures = Vec::new();
    for service in services {
        let Ok(Some(CFValue::Dictionary(stats))) = service.property("PerformanceStatistics") else {
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
    (average(&usages), average(&temperatures))
}

fn disk_sample() -> Option<DiskSample> {
    let services = matching_services("IOBlockStorageDriver").ok()?;
    let mut sample = DiskSample { read: 0, write: 0 };
    let mut found = false;
    for service in services {
        let Ok(Some(CFValue::Dictionary(stats))) = service.property("Statistics") else {
            continue;
        };
        if let Some(CFValue::Integer(value)) = stats.get("Bytes (Read)") {
            sample.read = sample.read.saturating_add((*value).max(0) as u64);
            found = true;
        }
        if let Some(CFValue::Integer(value)) = stats.get("Bytes (Write)") {
            sample.write = sample.write.saturating_add((*value).max(0) as u64);
            found = true;
        }
    }
    found.then_some(sample)
}

fn classify_temperature_keys(
    keys: Vec<FourCharCode>,
) -> (Vec<FourCharCode>, Vec<FourCharCode>, Vec<FourCharCode>) {
    let mut cpu = Vec::new();
    let mut gpu = Vec::new();
    let mut disk = Vec::new();
    for key in keys {
        let name = key.to_string();
        if name.starts_with("Tp")
            || name.starts_with("Te")
            || name.starts_with("Tf0")
            || name.starts_with("TC")
        {
            cpu.push(key);
        } else if name.starts_with("Tg")
            || name.starts_with("Tf1")
            || name.starts_with("Tf2")
            || name == "TGDD"
        {
            gpu.push(key);
        } else if name.starts_with("TH") {
            disk.push(key);
        }
    }
    (cpu, gpu, disk)
}

fn average_temperature(smc: Option<&smc::SMC>, keys: &[FourCharCode]) -> Option<f32> {
    let smc = smc?;
    let values: Vec<f32> = keys
        .iter()
        .filter_map(|key| smc.temperature(*key).ok())
        .filter(|value| (10.0..=120.0).contains(value))
        .map(|value| value as f32)
        .collect();
    average(&values)
}

fn average(values: &[f32]) -> Option<f32> {
    (!values.is_empty()).then(|| values.iter().sum::<f32>() / values.len() as f32)
}

fn cpu_sample() -> io::Result<CpuSample> {
    let mut info = HostCpuLoadInfo::default();
    let mut count = (mem::size_of::<HostCpuLoadInfo>() / mem::size_of::<c_int>()) as c_uint;
    let result = unsafe {
        host_statistics(
            mach_host_self(),
            HOST_CPU_LOAD_INFO,
            (&mut info as *mut HostCpuLoadInfo).cast(),
            &mut count,
        )
    };
    if result != 0 {
        return Err(io::Error::other(format!(
            "host_statistics failed: {result}"
        )));
    }
    Ok(CpuSample {
        total: info.ticks.iter().map(|v| *v as u64).sum(),
        idle: info.ticks[CPU_STATE_IDLE] as u64,
    })
}

fn memory_used(total_bytes: u64) -> io::Result<f32> {
    let mut info = VmStatistics64::default();
    let mut count = (mem::size_of::<VmStatistics64>() / mem::size_of::<c_int>()) as c_uint;
    let result = unsafe {
        host_statistics64(
            mach_host_self(),
            HOST_VM_INFO64,
            (&mut info as *mut VmStatistics64).cast(),
            &mut count,
        )
    };
    if result != 0 {
        return Err(io::Error::other(format!(
            "host_statistics64 failed: {result}"
        )));
    }
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    let available =
        (info.free_count as u64 + info.inactive_count as u64 + info.speculative_count as u64)
            * page_size;
    Ok(total_bytes.saturating_sub(available) as f32 * 100.0 / total_bytes as f32)
}

fn swap_used() -> io::Result<Option<f32>> {
    let mut usage = SwapUsage::default();
    sysctl_value("vm.swapusage", &mut usage)?;
    Ok((usage.total > 0).then(|| usage.used as f32 * 100.0 / usage.total as f32))
}

fn network_sample() -> io::Result<NetSample> {
    let mut addresses: *mut libc::ifaddrs = ptr::null_mut();
    if unsafe { libc::getifaddrs(&mut addresses) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let mut sample = NetSample { rx: 0, tx: 0 };
    let mut current = addresses;
    while !current.is_null() {
        let item = unsafe { &*current };
        if !item.ifa_addr.is_null()
            && unsafe { (*item.ifa_addr).sa_family as c_int } == libc::AF_LINK
            && !item.ifa_data.is_null()
        {
            let name = unsafe { CStr::from_ptr(item.ifa_name as *const c_char) }.to_string_lossy();
            let flags = item.ifa_flags as c_int;
            if name.starts_with("en") && flags & libc::IFF_UP != 0 {
                let data = unsafe { &*(item.ifa_data as *const libc::if_data) };
                sample.rx = sample.rx.saturating_add(data.ifi_ibytes as u64);
                sample.tx = sample.tx.saturating_add(data.ifi_obytes as u64);
            }
        }
        current = item.ifa_next;
    }
    unsafe { libc::freeifaddrs(addresses) };
    Ok(sample)
}

fn sysctl_u64(name: &str) -> io::Result<u64> {
    let mut value = 0u64;
    sysctl_value(name, &mut value)?;
    Ok(value)
}

fn sysctl_value<T>(name: &str, value: &mut T) -> io::Result<()> {
    let name = std::ffi::CString::new(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "sysctl name contains NUL"))?;
    let mut length = mem::size_of::<T>();
    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            (value as *mut T).cast::<c_void>(),
            &mut length,
            ptr::null_mut(),
            0,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
