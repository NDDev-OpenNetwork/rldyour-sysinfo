//! macOS memory and swap pressure from Mach VM statistics and sysctl.

use libc::{c_int, c_uint, c_void};
use std::io;
use std::mem;
use std::ptr;

const HOST_VM_INFO64: c_int = 4;

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
    fn host_statistics64(
        host: c_uint,
        flavor: c_int,
        info: *mut c_int,
        count: *mut c_uint,
    ) -> c_int;
}

pub struct Memory {
    total_bytes: u64,
}

pub struct Usage {
    /// Share of physical memory unavailable to new allocations, in percent.
    pub used: f32,
    /// Share of swap in use, in percent. `None` when the system has no swap.
    pub swap: Option<f32>,
}

impl Memory {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            total_bytes: sysctl_u64("hw.memsize")?,
        })
    }

    pub fn usage(&mut self) -> io::Result<Option<Usage>> {
        Ok(Some(Usage {
            used: memory_used(self.total_bytes)?,
            swap: swap_used()?,
        }))
    }
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
