# Runtime quality and platform verification

## Module boundaries

- `main` owns argument handling only; `transport` owns platform socket
  activation and standalone binding; `server` owns admission, cadence and
  publication. None of these implements hardware collection.
- `collector` selects a compile-time native `MetricsSource`; each platform
  separates CPU, memory, disk, network and temperature readers. NVML remains
  shared by Linux and Windows. The wire format is the stable v1 snapshot.
- macOS separates the main-actor AppKit UI, Sendable decoded values/formatting,
  and a serial-queue socket client. GNOME separates UI, formatting and async
  socket I/O; the Python client has no runtime dependencies.

## Resource and lifecycle contract

- At most 32 connected clients and 32 queued admissions. Oversized handshakes
  are dropped after 256 bytes; partial writes or full socket buffers disconnect
  a slow reader instead of delaying other clients.
- The handshake deadline covers the entire read, not each byte separately.
  Silence remains valid and selects the configured cadence.
- New clients receive the latest completed snapshot. A snapshot already due
  at their requested cadence is refreshed first. Connecting more clients
  cannot make the shared collector run faster than the 500 ms floor.
- Without clients, sampling stops. Managed Linux/macOS daemons exit after
  thirty idle seconds; standalone/Windows waits on a channel without polling.
- Socket ownership remains with the service manager. A standalone second
  instance refuses a live listener and never deletes a regular file or symlink.
- Native Create/Copy/Retain references and Mach send rights have explicit
  ownership. HID discovery is cached and refreshed every thirty seconds;
  unsupported readings remain null. AppleSMC/IOHID temperature and
  IOAccelerator properties are hardware-specific interfaces, not a promise
  of support for every future Apple model. No privileged helper is installed.
- Linux physical devices are rediscovered every thirty seconds. Per-device
  baselines prevent new lifetime counters from appearing as current traffic;
  read errors preserve the interval of the last successful observation.
- All clients bound a frame to 4096 bytes. UI clients reconnect after a failed
  connection, and cancelled GNOME operations cannot revive a disabled client.

## Release qualification

Run Rust tests and Clippy with/without NVIDIA, the extension static checker,
the GJS socket client regression, Python tests and native socket E2E. On macOS,
compile in Swift 6 mode with warnings as errors and target macOS 12; run both
format/model and dispatch socket checks. Test socket activation and idle exit
under the actual service managers, not only a manually launched process.

CI exercises Linux, Apple silicon/Intel macOS and Windows, including Windows with and without
NVIDIA. Rust 1.95 is the minimum version after updating Windows `sysinfo` to
0.39.6. The Python client targets CPython builds providing AF_UNIX; Windows
CPython currently lacks it, so Windows E2E uses .NET sockets. Windows does not
yet have a native tray client. GNOME metadata targets 46–50; a later Shell
version must be qualified before being added.

APT packages derive shared-library dependencies with `dpkg-shlibdeps` and
install a service pointing at `/usr/bin`. Source installs use `~/.local/bin`.
The two layouts must not be mixed through stale user overrides. A GNOME
extension installed into a running Wayland session may need the next login
before Shell discovers it; the installer does not restart that session.

APT publication follows the successful GitHub release through `workflow_run`
on the default branch. The `github-pages` environment remains restricted to
`main`; publication checks out and verifies the released tag/commit. A manual
repair also requires an existing release tag, preventing an unreleased main
snapshot from entering the stable APT repository.

The Debian maintainer-script regression runs the real helper in an isolated
`DPKG_ROOT`: a fresh install must create the global user-socket enablement link,
an enabled upgrade must preserve it, and an explicit disablement must survive
an upgrade. CI also installs the package on Ubuntu and queries systemctl's
global state rather than guessing the helper's link directory.

Resource measurements should report cadence, duration, CPU time per core and
the memory accounting method. RSS, Linux Pss and macOS physical footprint are
different quantities. Short unloaded measurements establish a regression
baseline, not a guarantee of zero leaks or permanent system stability.

## 0.3.1 qualification — 2026-10-08

The current release passed the native Rust, Clippy, Python, GJS, Swift and
package-lifecycle gates on Linux, macOS ARM/Intel and Windows. Installed Mac
and Ubuntu daemons passed the live socket regression for metric shape, cadence,
connection churn, bounded handshakes and silent peers. The Debian maintainer
script was checked for fresh-install socket enablement and preservation of an
explicit administrator disablement during upgrades. No process lists, user
files or private clipboard data are part of these checks.

The release archive and APT publication are tied to the merged stable commit.
Socket activation owns the listener; a daemon exiting after the idle period
with no clients is the documented low-resource state. Hardware support remains
model-dependent: unavailable temperatures or GPU readings are represented as
`null`, and a passing runtime check does not certify firmware or physical
hardware reliability.

## Primary references reviewed for 0.3.1

- [Rust bounded channels](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html)
  and [nonblocking Unix streams](https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html).
- [systemd socket activation](https://www.freedesktop.org/software/systemd/man/latest/sd_listen_fds.html)
  and [socket units](https://www.freedesktop.org/software/systemd/man/latest/systemd.socket.html).
- [Apple dispatch read sources](https://developer.apple.com/documentation/dispatch/dispatchsourceread)
  and [Mach IPC ownership](https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/KernelProgramming/Mach/Mach.html).
- [GNOME extension lifecycle rules](https://gjs.guide/extensions/review-guidelines/review-guidelines.html)
  and [GNOME asynchronous/resource-management practices](https://gjs.guide/extensions/review-guidelines/best-practices.html).
- [sysinfo selective refresh and CPU sampling guidance](https://docs.rs/sysinfo/latest/sysinfo/)
  and [NVIDIA NVML](https://docs.nvidia.com/deploy/nvml-api/nvml-api-reference.html).
