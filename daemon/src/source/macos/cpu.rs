//! macOS aggregate CPU busy time from Mach host statistics.

use crate::source::delta;
use libc::{c_int, c_uint};
use std::io;
use std::mem;

const HOST_CPU_LOAD_INFO: c_int = 3;
const CPU_STATE_MAX: usize = 4;
const CPU_STATE_IDLE: usize = 2;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct HostCpuLoadInfo {
    ticks: [u32; CPU_STATE_MAX],
}

unsafe extern "C" {
    fn mach_host_self() -> c_uint;
    fn host_statistics(host: c_uint, flavor: c_int, info: *mut c_int, count: *mut c_uint) -> c_int;
}

#[derive(Clone, Copy)]
struct Sample {
    total: u64,
    idle: u64,
}

pub struct Cpu {
    previous: Option<Sample>,
}

impl Cpu {
    pub fn new() -> Self {
        Self { previous: None }
    }

    /// Busy share of all cores since the previous call, in percent.
    ///
    /// The first call establishes the baseline and reports nothing, because a
    /// percentage needs two samples.
    pub fn usage(&mut self) -> io::Result<Option<f32>> {
        let current = read_ticks()?;
        let previous = self.previous.replace(current);

        Ok(previous.and_then(|previous| {
            let elapsed = delta(current.total, previous.total);
            if elapsed == 0 {
                return None;
            }
            let busy = elapsed - delta(current.idle, previous.idle).min(elapsed);
            Some(busy as f32 * 100.0 / elapsed as f32)
        }))
    }
}

fn read_ticks() -> io::Result<Sample> {
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
    Ok(Sample {
        total: info.ticks.iter().map(|v| *v as u64).sum(),
        idle: info.ticks[CPU_STATE_IDLE] as u64,
    })
}
