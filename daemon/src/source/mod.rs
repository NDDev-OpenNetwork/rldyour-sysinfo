//! Native metric collectors selected at compile time.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::LinuxCollector as PlatformCollector;
#[cfg(target_os = "macos")]
pub use macos::MacosCollector as PlatformCollector;
#[cfg(target_os = "windows")]
pub use windows::WindowsCollector as PlatformCollector;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("rldyour-sysinfod supports Linux, macOS, and Windows");
