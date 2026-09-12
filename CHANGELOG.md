# Changelog

## 0.2.0 — 2026-09-12

- Add a native macOS collector for CPU, memory, Apple GPU, disk, network and SMC temperatures.
- Add a native macOS menu bar client and per-user LaunchAgent installation.
- Add a Windows collector for CPU, memory, swap, network, temperatures and optional NVIDIA metrics.
- Split collectors into explicit `linux`, `macos` and `windows` modules.
- Make the local socket transport work on Windows 10 and newer.
- Verify daemon builds on Ubuntu, macOS and Windows in CI and publish platform archives.

## 0.1.0 — 2026-08-14

- Add the lightweight Linux daemon and GNOME Shell panel extension.
