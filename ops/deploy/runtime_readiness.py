"""Spend a shared readiness budget without replacing the HTTP client."""
import math
import time


def remaining(deadline, maximum):
    if deadline is None:
        return maximum
    try:
        valid = type(deadline) in (int, float) and math.isfinite(deadline)
    except OverflowError:
        valid = False
    if not valid:
        raise RuntimeError("readiness deadline is invalid")
    value = deadline - time.monotonic()
    if value <= 0:
        raise RuntimeError("readiness deadline expired")
    return min(value, maximum)


def read_body(response, maximum, deadline):
    # CPython HTTPResponse exposes read1 and its buffered socket through fp.raw.
    # Reject another transport shape instead of silently losing the socket bound.
    read = getattr(response, "read1", None)
    closed = getattr(response, "isclosed", None)
    if not callable(read) or not callable(closed):
        raise RuntimeError("readiness response cannot enforce its read boundary")
    content = bytearray()
    while True:
        timeout = remaining(deadline, 5)
        if closed():
            return bytes(content)
        socket = getattr(getattr(getattr(response, "fp", None), "raw", None), "_sock", None)
        setter = getattr(socket, "settimeout", None)
        if not callable(setter):
            raise RuntimeError("readiness response cannot enforce its socket boundary")
        setter(timeout)
        remaining(deadline, 5)
        data = read(min(65536, maximum - len(content) + 1))
        remaining(deadline, 5)
        if not data:
            return bytes(content)
        content.extend(data)
        if len(content) > maximum:
            raise RuntimeError("readiness response exceeds its body boundary")
