# Project overview

Provenance: repository source and verification on 2026-09-20.

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

The minimum supported Rust version is 1.85. Version 0.2.1 supports Linux,
macOS, and Windows 10 or newer at the daemon layer.
