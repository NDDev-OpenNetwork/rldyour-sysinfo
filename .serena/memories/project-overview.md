# Project overview

Provenance: repository source and verification on 2026-10-08.

`rldyour-sysinfo` publishes low-overhead host metrics over a versioned,
newline-delimited JSON protocol. The Rust daemon owns collection and cadence.
Linux renders it with a GNOME Shell extension; macOS renders it with a native
menu bar client. Windows currently exposes the daemon protocol for clients.

Platform collectors are isolated under `daemon/src/source/{linux,macos,windows}`.
Shared collection delegation lives in `collector.rs`, wire encoding in
`proto.rs`, and local transport/publication in `main.rs`. A missing hardware
capability must be emitted as JSON `null`; estimates must not masquerade as
measurements.

Cadence is negotiated per client through the `{"interval":N}` handshake:
1–60 seconds, or `0` for realtime — a 500 ms tick. The daemon serves the
minimum requested, and a new connection wakes the timing loop at once
(`recv_timeout` on the accept channel), so no client waits a full tick for
its first sample. On Windows, stock CPython never exposes `socket.AF_UNIX`;
the wire protocol there is exercised through .NET's `UnixDomainSocketEndPoint`
(`scripts/e2e-sample.ps1`).

The minimum supported Rust version is 1.95. The current stable release is
0.3.1 and supports Linux, macOS 12+, and Windows 10 or newer at the daemon
layer. GNOME Shell 46–50 is qualified; Wayland extensions need a new login
after installation because GNOME Shell cannot reload them in place.

The 0.3.1 Debian lifecycle fix enables the user socket on a fresh install,
preserves an administrator's explicit disablement on upgrades, and publishes
signed APT only after the matching stable GitHub release. Installed daemons
are socket-activated and stop sampling without clients; a zero-client daemon
exit is healthy, not a crash.
