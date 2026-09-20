# Changelog

## Unreleased

- Move the macOS socket to `~/Library/Application Support` (Caches is
  purgeable) and activate the daemon through a launchd `Sockets` entry, so it
  idles out like the systemd unit instead of running continuously.
- Restrict self-bound sockets to the owner (0600), remove stale socket files
  before binding, and keep `XDG_RUNTIME_DIR` from redirecting the socket on
  macOS and Windows.
- Count Linux CPU ticks once by excluding guest time already inside
  user/nice, and convert Windows network and volume counters into the
  bytes-per-second the protocol specifies; Windows now reports disk
  throughput and filters software adapters.
- Split the macOS and Windows collectors into the same per-domain module
  layout Linux uses, with one shared NVML reader honouring
  `RLDYOUR_SYSINFO_GPU=0` on both platforms.
- Check the protocol version in every client, align the menu-bar app's
  formatting and reconnect defaults with the extension, and send the Python
  handshake as an integer.
- Enable the APT package's user socket on install via maintainer scripts and
  point its service at `/usr/bin`; write proper metadata into the repository
  `Release` file.
- Register the Windows daemon through the `Run` key as a windowless process
  instead of a console-flashing Startup shim.

## 0.2.0 — 2026-09-12

- Publish the Rust daemon on crates.io and a dependency-free protocol client
  on PyPI.
- Add a signed APT repository for Ubuntu amd64, hosted on GitHub Pages.

- Add a native macOS collector for CPU, memory, Apple GPU, disk, network and SMC temperatures.
- Add a native macOS menu bar client and per-user LaunchAgent installation.
- Add a Windows collector for CPU, memory, swap, network, temperatures and optional NVIDIA metrics.
- Split collectors into explicit `linux`, `macos` and `windows` modules.
- Make the local socket transport work on Windows 10 and newer.
- Verify daemon builds on Ubuntu, macOS and Windows in CI and publish platform archives.

## 0.1.0 — 2026-08-14

- Add the lightweight Linux daemon and GNOME Shell panel extension.
