"""Bound Linux cgroup and socket observations without signalling or connecting."""
import math
import os
from pathlib import Path
import re
import stat
from time import monotonic


MAX_TASKS = 4096
MAX_GROUPS = 256
MAXIMUM = 65536
MAX_NETWORK = 1024 * 1024


def budget(timeout):
    if type(timeout) not in (int, float) or not math.isfinite(timeout) or not 0 < timeout <= 300:
        raise ValueError("restore observation timeout is invalid")
    return monotonic() + timeout


def check(deadline):
    if monotonic() >= deadline:
        raise RuntimeError("restore observation deadline expired")


def exact(path, *, missing=False):
    if not isinstance(path, Path) or not path.is_absolute():
        raise ValueError("restore observation requires an absolute path")
    for part in (path, *path.parents):
        if part.is_symlink():
            raise ValueError("restore observation cannot follow redirected paths")
    if path.resolve(strict=not missing) != path:
        raise ValueError("restore observation path is not exact")


def read(path, maximum, deadline):
    check(deadline)
    exact(path)
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        before = os.fstat(stream.fileno())
        if not stat.S_ISREG(before.st_mode):
            raise ValueError("restore observation input is not a regular file")
        value = stream.read(maximum + 1)
        after = os.fstat(stream.fileno())
        current = path.lstat()
        if (len(value) > maximum or (before.st_dev, before.st_ino) != (after.st_dev, after.st_ino)
                or (after.st_dev, after.st_ino) != (current.st_dev, current.st_ino)):
            raise ValueError("restore observation input changed or exceeded its boundary")
    check(deadline)
    if not value.isascii():
        raise ValueError("restore observation input is not ASCII")
    return value


def group_path(value):
    if (not isinstance(value, str) or not value.startswith("/") or len(value) > 512
            or not re.fullmatch(r"/(?:[A-Za-z0-9_@.-]+/)*[A-Za-z0-9_@.-]+", value)
            or any(part in (".", "..") for part in value.split("/"))):
        raise ValueError("restore cgroup path is invalid")
    return value


def task_list(path, deadline):
    raw = read(path, MAXIMUM, deadline).decode("ascii")
    result = set()
    for value in raw.splitlines():
        if not re.fullmatch(r"[1-9][0-9]{0,9}", value) or not 0 < int(value) < 2**31 or int(value) in result:
            raise ValueError("restore cgroup task list is ambiguous")
        result.add(int(value))
    if len(result) > MAX_TASKS:
        raise ValueError("restore cgroup contains too many tasks")
    return result


def groups(root, selected, deadline):
    check(deadline)
    exact(root)
    if not root.is_dir():
        raise ValueError("restore cgroup root is not a directory")
    if not read(root / "cgroup.controllers", MAXIMUM, deadline).strip():
        raise ValueError("restore observation requires cgroup v2")
    start = root / selected.lstrip("/")
    exact(start, missing=True)
    if not start.exists():
        return {}
    pending, found = [start], {}
    while pending:
        check(deadline)
        current = pending.pop()
        exact(current)
        if not current.is_dir() or len(found) >= MAX_GROUPS:
            raise ValueError("restore cgroup directory inventory exceeds its boundary")
        group = "/" + current.relative_to(root).as_posix()
        found[group] = task_list(current / "cgroup.procs", deadline)
        with os.scandir(current) as entries:
            count = 0
            for entry in entries:
                check(deadline)
                count += 1
                if count > MAXIMUM or entry.is_symlink():
                    raise ValueError("restore cgroup inventory is ambiguous")
                if entry.is_dir(follow_symlinks=False):
                    pending.append(Path(entry.path))
                    if len(pending) + len(found) > MAX_GROUPS:
                        raise ValueError("restore cgroup directory inventory exceeds its boundary")
    return found


def start_time(raw, pid):
    text = raw.decode("ascii").strip()
    end = text.rfind(")")
    if not text.startswith(str(pid) + " (") or end < 0:
        raise ValueError("restore process stat identity differs")
    fields = text[end + 1:].split()
    if len(fields) < 20 or not re.fullmatch(r"[A-Za-z]", fields[0]) or not fields[19].isdecimal() or int(fields[19]) <= 0:
        raise ValueError("restore process stat is incomplete")
    return int(fields[19])


def owner(raw):
    values = [line.partition(":")[2].split() for line in raw.decode("ascii").splitlines() if line.startswith("Uid:")]
    if (len(values) != 1 or len(values[0]) != 4 or any(not part.isdecimal() for part in values[0])
            or len(set(values[0])) != 1 or not 0 <= int(values[0][0]) < 2**32):
        raise ValueError("restore process user identity is incomplete or changing")
    return int(values[0][0])


def membership(raw):
    lines = raw.decode("ascii").splitlines()
    if len(lines) != 1 or not lines[0].startswith("0::"):
        raise ValueError("restore process cgroup identity is ambiguous")
    return group_path(lines[0][3:])


def process_snapshot(control_group, *, timeout, proc_root=Path("/proc"), cgroup_root=Path("/sys/fs/cgroup")):
    deadline = budget(timeout)
    selected = group_path(control_group)
    exact(proc_root)
    before = groups(cgroup_root, selected, deadline)
    tasks = [(pid, group) for group, members in before.items() for pid in sorted(members)]
    if len(tasks) > MAX_TASKS or len({pid for pid, _ in tasks}) != len(tasks):
        raise ValueError("restore cgroup task inventory is ambiguous")
    result = []
    for pid, group in tasks:
        base = proc_root / str(pid)
        ticks = start_time(read(base / "stat", MAXIMUM, deadline), pid)
        uid = owner(read(base / "status", MAXIMUM, deadline))
        actual_group = membership(read(base / "cgroup", MAXIMUM, deadline))
        if actual_group != group:
            raise ValueError("restore process moved outside its observed cgroup")
        if (start_time(read(base / "stat", MAXIMUM, deadline), pid) != ticks
                or owner(read(base / "status", MAXIMUM, deadline)) != uid
                or membership(read(base / "cgroup", MAXIMUM, deadline)) != group):
            raise ValueError("restore process identity changed during observation")
        result.append({"pid": pid, "uid": uid, "start_ticks": ticks, "control_group": group})
    if groups(cgroup_root, selected, deadline) != before:
        raise ValueError("restore cgroup task inventory changed during observation")
    check(deadline)
    return result


def socket_table(raw, width):
    lines = raw.decode("ascii").splitlines()
    if not lines:
        raise ValueError("restore socket table is empty")
    header = lines[0].split()
    if len(header) < 4 or header[:2] != ["sl", "local_address"] or header[2] not in ("rem_address", "remote_address") or header[3] != "st":
        raise ValueError("restore socket table header differs")
    if len(lines) > 8193:
        raise ValueError("restore socket table exceeds its row boundary")
    slots, listeners = set(), set()
    address = re.compile(r"[0-9A-Fa-f]{" + str(width) + r"}:[0-9A-Fa-f]{4}")
    for line in lines[1:]:
        fields = line.split()
        if (len(fields) < 10 or not re.fullmatch(r"[0-9]+:", fields[0]) or fields[0] in slots
                or not address.fullmatch(fields[1]) or not address.fullmatch(fields[2])
                or not re.fullmatch(r"[0-9A-Fa-f]{2}", fields[3])
                or not fields[7].isdecimal() or not fields[9].isdecimal()):
            raise ValueError("restore socket table row is malformed")
        slots.add(fields[0])
        if fields[3].upper() == "0A":
            listeners.add(int(fields[1].rsplit(":", 1)[1], 16))
    return listeners


def listening_ports(ports, *, timeout, proc_root=Path("/proc")):
    deadline = budget(timeout)
    if (not isinstance(ports, (list, tuple, set)) or not 0 < len(ports) <= 4
            or any(type(port) is not int or not 0 < port <= 65535 for port in ports)
            or len(set(ports)) != len(ports)):
        raise ValueError("restore listener ports are invalid")
    exact(proc_root)
    found = set()
    # /proc/net and /proc/self are kernel symlinks; use this process's exact namespace.
    network = proc_root / str(os.getpid()) / "net" if proc_root == Path("/proc") else proc_root / "net"
    for name, width in (("tcp", 8), ("tcp6", 32)):
        raw = read(network / name, MAX_NETWORK, deadline)
        found.update(socket_table(raw, width))
        check(deadline)
    return sorted(set(ports) & found)
