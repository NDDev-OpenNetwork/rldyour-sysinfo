import json

import pytest

from rldyour_sysinfo import _decode, samples


def test_decode_accepts_protocol_v1():
    value = {"v": 1, "cpu": {}, "memory": {}, "gpu": {}, "disk": {}, "net": {}}
    assert _decode(json.dumps(value).encode()) == value


def test_decode_rejects_unknown_shape():
    with pytest.raises(ValueError):
        _decode(b'{"v":2}')


def test_decode_accepts_extra_fields_within_v1():
    value = {"v": 1, "cpu": {}, "memory": {}, "gpu": {}, "disk": {}, "net": {}, "uptime": 42}
    assert _decode(json.dumps(value).encode()) == value


def test_interval_is_bounded_before_connecting():
    with pytest.raises(ValueError):
        next(samples(61))
    with pytest.raises(ValueError):
        next(samples(-1))
    for value in (True, 0.5, "5"):
        with pytest.raises(ValueError):
            next(samples(value))


@pytest.mark.parametrize("version", [True, 1.0, "1", 2])
def test_version_is_a_protocol_integer(version):
    value = {"v": version, "cpu": {}, "memory": {}, "gpu": {}, "disk": {}, "net": {}}
    with pytest.raises(ValueError):
        _decode(json.dumps(value).encode())


def test_metric_sections_must_be_objects():
    with pytest.raises(ValueError):
        _decode(b'{"v":1,"cpu":null,"memory":{},"gpu":{},"disk":{},"net":{}}')
