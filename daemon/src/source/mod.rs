//! Native metric collectors selected at compile time.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) mod nvidia;

#[cfg(target_os = "linux")]
pub use linux::LinuxCollector as PlatformCollector;
#[cfg(target_os = "macos")]
pub use macos::MacosCollector as PlatformCollector;
#[cfg(target_os = "windows")]
pub use windows::WindowsCollector as PlatformCollector;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("rldyour-sysinfod supports Linux, macOS, and Windows");

/// A counter difference that treats a reset as zero.
///
/// Kernel counters only ever move forward, but a hot-unplugged device or a
/// 32-bit wraparound can make the new reading smaller; saturating keeps a
/// reset from becoming a huge false spike.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn delta(current: u64, previous: u64) -> u64 {
    current.saturating_sub(previous)
}

/// Bytes per second over a real elapsed interval.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn rate(bytes: u64, seconds: f64) -> u64 {
    if seconds <= 0.0 {
        0
    } else {
        (bytes as f64 / seconds) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_tolerate_resets_and_zero_intervals() {
        assert_eq!(delta(10, 4), 6);
        assert_eq!(delta(4, 10), 0);
        assert_eq!(rate(1024, 2.0), 512);
        assert_eq!(rate(1024, 0.0), 0);
    }
}
