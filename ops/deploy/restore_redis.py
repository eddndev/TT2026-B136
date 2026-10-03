"""Bounded removal of captured sessions while preserving Redis controls."""
import json
import math
from pathlib import Path
import re
import stat
import subprocess

from restore_commands import run


MAXIMUM_OUTPUT = 65536
NAMESPACES = {
    "session": "sessions_removed",
    "challenge": "challenges_removed",
    "certificate-login": "certificate_logins_removed",
}
FAILED = "Redis restore-session invalidation did not complete"


def _integer(value, maximum):
    return type(value) is int and 0 < value <= maximum


def _path(path, *, directory=False):
    if not isinstance(path, Path) or not path.is_absolute():
        raise ValueError("restore Redis paths must be explicit absolute paths")
    information = path.lstat()
    if path.resolve(strict=True) != path:
        raise ValueError("restore Redis paths must not redirect")
    mode = information.st_mode
    if directory:
        valid = stat.S_ISDIR(mode) and stat.S_IMODE(mode) == 0o700
    else:
        valid = stat.S_ISREG(mode) and bool(mode & 0o111)
    if not valid:
        raise ValueError("restore Redis path type or permissions differ")


def _validate(redis_cli, host, port, password, expected_pid,
              expected_directory, max_scan_calls, max_keys, batch_size, timeout):
    if (host != "127.0.0.1" or not isinstance(host, str)
            or not _integer(port, 65535) or not _integer(expected_pid, 2147483647)
            or not _integer(max_scan_calls, 1000) or not _integer(max_keys, 10000)
            or not _integer(batch_size, 100)
            or type(timeout) not in (int, float) or not 0 < timeout <= 10
            or not math.isfinite(timeout)
            or not isinstance(password, str) or not password or "\0" in password):
        raise ValueError("restore Redis target or limits are invalid")
    try:
        _path(redis_cli)
        _path(expected_directory, directory=True)
    except (OSError, ValueError, RuntimeError):
        raise ValueError("restore Redis paths are invalid") from None


def _nonfinite(_value):
    raise ValueError("invalid Redis response")


class _Connection:
    def __init__(self, redis_cli, host, port, password, timeout, max_scan_calls):
        self.arguments = [str(redis_cli), "-h", host, "-p", str(port),
                          "-n", "0", "-2"]
        self.environment = {"REDISCLI_AUTH": password, "LANG": "C", "LC_ALL": "C"}
        self.timeout = timeout
        self.maximum_scans = max_scan_calls
        self.scans = 0

    def command(self, *arguments):
        try:
            raw = arguments[:1] == ("INFO",)
            output = "--raw" if raw else "--json"
            result = run([*self.arguments, output, *arguments], env=self.environment,
                         timeout=self.timeout, maximum=MAXIMUM_OUTPUT)
            if (result.returncode != 0 or not isinstance(result.stdout, bytes)
                    or not 0 < len(result.stdout) <= MAXIMUM_OUTPUT):
                raise ValueError("invalid Redis response")
            value = result.stdout.decode("utf-8")
            return value if raw else json.loads(value, parse_constant=_nonfinite)
        except (OSError, ValueError, RuntimeError, RecursionError,
                subprocess.SubprocessError):
            raise RuntimeError(FAILED) from None

    def verify(self, expected_pid, expected_directory):
        if self.command("PING") != "PONG":
            raise RuntimeError(FAILED)
        raw = self.command("INFO", "server")
        if not isinstance(raw, str):
            raise RuntimeError(FAILED)
        fields = {}
        for line in raw.splitlines():
            if not line or line.startswith("#"):
                continue
            name, separator, value = line.partition(":")
            if not separator or not name or name in fields:
                raise RuntimeError(FAILED)
            fields[name] = value
        if fields.get("process_id") != str(expected_pid):
            raise RuntimeError(FAILED)
        if self.command("CONFIG", "GET", "dir") != ["dir", str(expected_directory)]:
            raise RuntimeError(FAILED)

    def collect(self, namespace, collected, max_keys, *, require_empty=False):
        cursor = "0"
        keys = set()
        pattern = "identity:" + namespace + ":*"
        exact = re.compile("identity:" + namespace + r":[a-f0-9]{64}")
        while True:
            if self.scans >= self.maximum_scans:
                raise RuntimeError(FAILED)
            self.scans += 1
            page = self.command("SCAN", cursor, "MATCH", pattern, "COUNT", "100")
            if (not isinstance(page, list) or len(page) != 2
                    or not isinstance(page[0], str)
                    or not re.fullmatch(r"0|[1-9][0-9]{0,19}", page[0])
                    or int(page[0]) > 18446744073709551615
                    or not isinstance(page[1], list)):
                raise RuntimeError(FAILED)
            cursor, observed = page
            for key in observed:
                if not isinstance(key, str) or not exact.fullmatch(key):
                    raise RuntimeError(FAILED)
                if require_empty:
                    raise RuntimeError(FAILED)
                keys.add(key)
                collected.add(key)
                if len(collected) > max_keys:
                    raise RuntimeError(FAILED)
            if cursor == "0":
                return keys


def invalidate_sessions(*, redis_cli, host, port, password, expected_pid,
                        expected_directory, max_scan_calls, max_keys,
                        batch_size, timeout):
    """Invalidate exact authentication keys with the caller's writers closed.

    All initial traversals finish before deletion. A failed command or final
    traversal may follow partial deletion; retrying the same target is safe.
    Persistence and service admission remain the caller's responsibility.
    """
    _validate(redis_cli, host, port, password, expected_pid, expected_directory,
              max_scan_calls, max_keys, batch_size, timeout)
    connection = _Connection(redis_cli, host, port, password, timeout, max_scan_calls)
    connection.verify(expected_pid, expected_directory)
    collected = set()
    inventories = {
        namespace: connection.collect(namespace, collected, max_keys)
        for namespace in NAMESPACES
    }
    # Each final namespace requires at least one additional SCAN call.
    if connection.scans + len(NAMESPACES) > max_scan_calls:
        raise RuntimeError(FAILED)
    removed = {}
    for namespace, keys in inventories.items():
        ordered = sorted(keys)
        count = 0
        for start in range(0, len(ordered), batch_size):
            batch = ordered[start:start + batch_size]
            deleted = connection.command("DEL", *batch)
            if type(deleted) is not int or not 0 <= deleted <= len(batch):
                raise RuntimeError(FAILED)
            count += deleted
        removed[NAMESPACES[namespace]] = count
    for namespace in NAMESPACES:
        connection.collect(namespace, collected, max_keys, require_empty=True)
    return removed
