# rldyour-sysinfo

System load in the GNOME top bar, immediately left of the clock: processor,
memory, graphics, temperatures, disk and network — refreshed every five
seconds, at a cost small enough to forget about.

![The indicator in the top bar](docs/panel.png)

## Why it is two pieces

The GNOME top bar can only be extended from inside `gnome-shell`, and that
process runs JavaScript. Rust cannot be placed in the panel at all. So the
panel widget is a GJS extension, and everything that costs anything — opening
kernel files, parsing them, talking to the NVIDIA driver — lives in a separate
Rust daemon. They speak over a private unix socket in the user's runtime
directory.

The split is not only a workaround. Under Wayland the shell cannot be restarted
without logging out, so any change to extension code costs a session. Keeping
the extension thin and stable, and putting everything that evolves into a
daemon that restarts in milliseconds, is what makes the thing maintainable.

### What the design deliberately avoids

- **No async runtime.** One timer at 0.2 Hz does not pay for `tokio` or
  `async-io`. The daemon is one sleeping thread plus one accept thread.
- **No D-Bus stack.** `zbus` spawns a thread per connection and its own
  executor. For pushing under two hundred bytes every five seconds, a plain
  `UnixListener` from the standard library costs nothing and depends on nothing.
- **No `sysinfo` crate.** It wants an explicit refresh per call and allocates a
  full process list to answer questions this widget never asks. The daemon reads
  `/proc` and `/sys` directly, into buffers it allocates once.
- **No polling in the shell.** The cadence belongs to the daemon; the extension
  only waits on the next line. It runs no timer while connected, which also
  removes the most common cause of leaks in shell extensions.

## Cost

Measured on the development machine, resident private memory (`Pss`):

| Configuration | Pss | Threads | Binary |
|---|---|---|---|
| Daemon without GPU metrics | ~0.6 MB | 2 | 375 KB |
| Daemon with NVIDIA metrics | ~19.6 MB | 3 | 484 KB |

The entire difference is NVML. The proprietary NVIDIA driver publishes no
utilisation or temperature through `procfs` or `sysfs` and binds no hwmon node,
so NVML is the only interface that answers, and it maps about nineteen
megabytes of driver-side state. Set `RLDYOUR_SYSINFO_GPU=0` in the service
environment, or build with `--no-default-features`, and that cost disappears
along with the GPU readings.

## Install

Requires a Rust toolchain and GNOME Shell 48 or newer.

```sh
./install.sh
```

This builds the daemon into `~/.local/bin`, installs a socket-activated user
service, and copies the extension into place. Nothing needs root.

The daemon starts on the first connection and stops again thirty seconds after
the last client disconnects, so it consumes nothing while the extension is off.

Because Wayland cannot restart the shell in place, log out and back in, then:

```sh
gnome-extensions enable rldyour-sysinfo@nddev-opennetwork
```

To remove everything: `./uninstall.sh`.

## Configuration

Both variables belong in `~/.config/systemd/user/rldyour-sysinfod.service`.

| Variable | Default | Meaning |
|---|---|---|
| `RLDYOUR_SYSINFO_INTERVAL` | `5` | Seconds between samples |
| `RLDYOUR_SYSINFO_GPU` | unset | Set to `0` to skip NVML entirely |

## Protocol

One newline-terminated JSON object per tick. Every metric is always present;
one the host cannot supply is `null`, so a client never distinguishes "missing"
from "unsupported". `v` is incremented only on an incompatible change.

```json
{"v":1,"cpu":{"usage":55.4,"temp":88.6},"memory":{"used":54.7,"swap":4.2},
 "gpu":{"usage":7.0,"memory":9.6,"temp":69.0},
 "disk":{"read":10628,"write":3789480,"temp":47.8},
 "net":{"rx":18360,"tx":342578}}
```

Percentages are per cent, temperatures are degrees Celsius, disk and network
figures are bytes per second.

## Checks

```sh
cd daemon && cargo test && cargo clippy --all-targets --all-features -- -D warnings
gjs -m extension/tests/smoke.js   # needs the daemon reachable
```

## Licence

AGPL-3.0-or-later.

Note that this places the extension outside what extensions.gnome.org accepts:
the portal requires every extension to be distributable under GPL-2.0-or-later,
which AGPL-3.0 is not compatible with. The portal separately forbids shipping
binaries, which this project needs. Distribution is therefore from this
repository.
