#!/usr/bin/env python3
# rldyour-sysinfo — end-to-end protocol check against a live daemon.
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Connects at a 1-second cadence, reads two samples, and asserts the wire
# shape the clients rely on. The second tick must carry a CPU figure: the
# first can legitimately be null while the counters establish a baseline.

import rldyour_sysinfo

for count, sample in enumerate(rldyour_sysinfo.samples(interval=1), start=1):
    assert sample["v"] == 1, f"unexpected protocol version {sample['v']}"
    print(
        f"sample {count}: cpu={sample['cpu']['usage']} "
        f"mem={sample['memory']['used']} "
        f"rx={sample['net']['rx']}"
    )
    if count == 2:
        assert sample["cpu"]["usage"] is not None, "second tick lacks CPU usage"
        assert sample["memory"]["used"] is not None, "second tick lacks memory usage"
        break
else:
    raise SystemExit("daemon closed the stream before two samples")

print("e2e ok")
