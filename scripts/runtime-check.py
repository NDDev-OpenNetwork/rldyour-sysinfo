#!/usr/bin/env python3
"""Live socket regression checks; only synthetic requests and host metrics.

Run against a test daemon with --socket PATH. No daemon is started or stopped,
no settings are changed and no filesystem or process-list data is collected.
"""
import argparse
import json
import socket
import statistics
import time


def connect(path, opening=b'{"interval":0}\n'):
    peer = socket.socket(socket.AF_UNIX)
    peer.settimeout(5)
    peer.connect(path)
    if opening:
        peer.sendall(opening)
    return peer


def frame(stream):
    line = stream.readline(4097)
    assert line.endswith(b"\n") and len(line) <= 4096, "bounded complete frame"
    sample = json.loads(line)
    assert sample["v"] == 1
    for section, fields in {"cpu": ("usage", "temp"), "memory": ("used", "swap"),
                            "gpu": ("usage", "memory", "temp"), "disk": ("read", "write", "temp"),
                            "net": ("rx", "tx")}.items():
        assert set(fields).issubset(sample[section])
        for key in fields:
            value = sample[section][key]
            assert value is None or (type(value) in (int, float) and value >= 0)
    return sample


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--socket", required=True)
    args = parser.parse_args()
    with connect(args.socket) as healthy, healthy.makefile("rb") as stream:
        frame(stream)
        frame(stream)
        times = []
        for _ in range(4):
            start = time.monotonic()
            frame(stream)
            times.append(time.monotonic() - start)
        assert 0.40 <= statistics.median(times) < 1.5, times

        # New clients get the cached completed sample, without forcing extra
        # publication to the original client on every connection.
        peers = []
        try:
            start = time.monotonic()
            for _ in range(20):
                peer = connect(args.socket)
                peers.append(peer)
                with peer.makefile("rb") as other:
                    frame(other)
            elapsed = time.monotonic() - start
            after = []
            for _ in range(5):
                start = time.monotonic()
                frame(stream)
                after.append(time.monotonic() - start)
            assert statistics.median(after) >= 0.40, (after, elapsed)
        finally:
            for peer in peers:
                peer.close()
            healthy.settimeout(5)

        with connect(args.socket, b"x" * 256) as oversized:
            assert oversized.recv(1) == b"", "oversized handshake must be dropped"
        with connect(args.socket, b"") as silent, silent.makefile("rb") as other:
            start = time.monotonic()
            frame(other)
            assert time.monotonic() - start < 1.5
        frame(stream)
    print("PASS: v1 metrics, realtime cadence, connection churn, bounded handshake, silent peer")


if __name__ == "__main__":
    main()
