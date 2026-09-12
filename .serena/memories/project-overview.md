# Project overview

Provenance: repository source and verification on 2026-09-12.

`rldyour-sysinfo` publishes low-overhead host metrics over a versioned,
newline-delimited JSON protocol. The Rust daemon owns collection and cadence.
Linux renders it with a GNOME Shell extension; macOS renders it with a native
menu bar client. Windows currently exposes the daemon protocol for clients.

Platform collectors are isolated under `daemon/src/source/{linux,macos,windows}`.
Shared collection delegation lives in `collector.rs`, wire encoding in
`proto.rs`, and local transport/publication in `main.rs`. A missing hardware
capability must be emitted as JSON `null`; estimates must not masquerade as
measurements.

The minimum supported Rust version is 1.85. Version 0.2.0 supports Linux,
macOS, and Windows 10 or newer at the daemon layer.
