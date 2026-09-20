# rldyour-sysinfo Python client

Typed, dependency-free client for the local JSON protocol exposed by the
`rldyour-sysinfod` Rust daemon.

```python
from rldyour_sysinfo import samples

for sample in samples(interval=2):
    print(sample["gpu"])
```

`interval` is the cadence in seconds, 0–60; `0` selects realtime, a 500 ms
tick. The command `rldyour-sysinfo --once` prints one sample as JSON.

Linux and macOS. CPython on Windows builds without `socket.AF_UNIX`, so this
client cannot reach the daemon there — drive the same protocol through .NET's
`UnixDomainSocketEndPoint` instead (see `scripts/e2e-sample.ps1`).

Install the daemon from the
[main repository](https://github.com/NDDev-OpenNetwork/rldyour-sysinfo).
