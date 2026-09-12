# rldyour-sysinfo Python client

Typed, dependency-free client for the local JSON protocol exposed by the
`rldyour-sysinfod` Rust daemon.

```python
from rldyour_sysinfo import samples

for sample in samples(interval=2):
    print(sample["gpu"])
```

The command `rldyour-sysinfo --once` prints one sample as JSON. Install the
daemon from the [main repository](https://github.com/NDDev-OpenNetwork/rldyour-sysinfo).
