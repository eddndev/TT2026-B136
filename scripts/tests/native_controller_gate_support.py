"""Owned local service fixtures with identity-checked native gate cleanup."""
import hashlib
import json
import os
from pathlib import Path
import pwd
import re
import shutil
import socket
import stat
import sys
import tempfile
import time
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
from backup_manifest_files import sync_directory
from controller_gate import UNITS
from controller_gate_target import PROPERTIES
from restore_commands import run
from restore_quiesce_observe import listening_ports, process_snapshot

OPT_IN = "TT_CONTROLLER_GATE_NATIVE"


def require(value, message):
    if not value:
        raise RuntimeError(message)


def node(path):
    before = path.lstat()
    value = None
    if stat.S_ISREG(before.st_mode):
        require(before.st_size <= 65536 and before.st_nlink == 1, "fixture file exceeds its boundary")
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(descriptor, "rb") as stream:
            require((os.fstat(stream.fileno()).st_dev, os.fstat(stream.fileno()).st_ino)
                    == (before.st_dev, before.st_ino), "fixture file changed while opening")
            value = stream.read(65537)
        require(len(value) == before.st_size, "fixture file changed while reading")
    elif stat.S_ISLNK(before.st_mode):
        value = os.readlink(path)
    else:
        require(stat.S_ISDIR(before.st_mode), "fixture contains an unsupported object")
    after = path.lstat()
    require((before.st_dev, before.st_ino, before.st_ctime_ns)
            == (after.st_dev, after.st_ino, after.st_ctime_ns), "fixture object changed during inspection")
    return (after.st_dev, after.st_ino, after.st_uid, after.st_mode, after.st_mtime_ns, value)


def snapshot(directory, exclude=()):
    result, pending, total = {}, [directory], 0
    while pending:
        path = pending.pop()
        key = str(path.relative_to(directory))
        if key in exclude:
            continue
        value = node(path)
        result[key] = value[:4] if path == directory else value
        require(len(result) <= 256, "fixture directory inventory exceeds its boundary")
        total += len(value[-1]) if isinstance(value[-1], (bytes, str)) else 0
        require(total <= 1024 * 1024, "fixture directory bytes exceed their boundary")
        if stat.S_ISDIR(value[3]):
            with os.scandir(path) as entries:
                for entry in entries:
                    pending.append(Path(entry.path))
                    require(len(pending) + len(result) <= 256, "fixture directory inventory is too large")
    return result


def private_file(path, data):
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    sync_directory(path.parent)


def encoded(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("ascii")


def read_json(path):
    value = node(path)[-1]
    require(isinstance(value, bytes) and value.isascii(), "fixture receipt is not bounded ASCII")
    return json.loads(value)


def fingerprint(path, maximum=65536):
    before = path.stat()
    require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= maximum, "fixture fingerprint size is invalid")
    digest = hashlib.sha256()
    amount = 0
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        opened = os.fstat(stream.fileno())
        require((opened.st_dev, opened.st_ino, opened.st_mode) == (before.st_dev, before.st_ino, before.st_mode),
                "fixture fingerprint identity changed while opening")
        for block in iter(lambda: stream.read(65536), b""):
            amount += len(block)
            require(amount <= maximum, "fixture fingerprint exceeds its byte boundary")
            digest.update(block)
    after = path.stat()
    require(amount == before.st_size and (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns)
            == (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns), "fixture fingerprint input changed")
    return {"path": str(path), "size": before.st_size, "sha256": digest.hexdigest()}


def safe_path(path):
    require(path.is_absolute() and path.resolve(strict=True) == path
            and re.fullmatch(r"/[A-Za-z0-9_./-]+", str(path)), "native fixture requires an exact simple path")
    return path


class Harness:
    def __init__(self):
        require(sys.platform == "linux" and os.environ.get(OPT_IN) == "1", "native gate requires explicit Linux opt-in")
        self.uid = os.getuid()
        require(self.uid > 0 and self.uid == os.geteuid() and os.getgid() == os.getegid(), "use an unprivileged account")
        account = pwd.getpwuid(self.uid)
        require(account.pw_name == "tt-runner", "native gate is restricted to the disposable tt-runner account")
        self.home = safe_path(Path(account.pw_dir))
        self.environment = {key: os.environ[key] for key in ("HOME", "XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS")}
        require(self.environment == {"HOME": str(self.home), "XDG_RUNTIME_DIR": f"/run/user/{self.uid}",
            "DBUS_SESSION_BUS_ADDRESS": f"unix:path=/run/user/{self.uid}/bus"}, "native gate manager context differs")
        self.environment["LC_ALL"] = "C"
        self.controller = safe_path(Path(shutil.which("systemctl") or "/missing-systemctl").resolve(strict=True))
        self.units_directory = safe_path(self.home / ".config/systemd/user")
        info = self.units_directory.stat()
        require(info.st_uid == self.uid and stat.S_IMODE(info.st_mode) == 0o700,
                "native gate requires a preexisting owned 0700 unit directory; it never changes that mode")
        self.directory_identity = (info.st_dev, info.st_ino, info.st_uid, info.st_mode)
        require(not any(os.path.lexists(self.units_directory / name) for name in
            (*UNITS, "service.d", "qadra-.service.d", *(unit + ".d" for unit in UNITS))), "managed unit paths already exist")
        initial = self.show()
        require(all(row["LoadState"] == "not-found" and not row["FragmentPath"] and not row["DropInPaths"]
            and row["MainPID"] == row["ControlPID"] == "0" and not row["ControlGroup"]
            and row["ActiveState"] == "inactive" and row["SubState"] == "dead"
            for row in initial.values()), "refusing to use existing Qadra units")
        self.foreign = snapshot(self.units_directory, UNITS)
        require(not any(Path(name).name in UNITS or (isinstance(value[-1], str)
            and Path(value[-1]).name in UNITS) for name, value in self.foreign.items()),
            "managed unit aliases or dependency links already exist")
        self.root, self.created, self.tasks, self.reservations = None, {}, {}, []
        self.masks = {}
        self.artifacts = {}
        self.operation = str(uuid.uuid4())
        self.completed = False

    def command(self, *arguments):
        return run([self.controller, "--user", *arguments], env=self.environment,
                   timeout=10, maximum=65536, text=True).stdout

    def show(self):
        properties = (*PROPERTIES, "MemoryMax", "RuntimeMaxUSec")
        raw = self.command("show", *UNITS, "--property=" + ",".join(properties))
        result = {}
        for block in raw.strip().split("\n\n"):
            row = {}
            for line in block.splitlines():
                key, separator, value = line.partition("=")
                require(separator and key not in row, "ambiguous native unit observation")
                row[key] = value
            require(set(row) == set(properties) and row["Id"] in UNITS and row["Id"] not in result,
                    "incomplete native unit observation")
            result[row["Id"]] = row
        require(set(result) == set(UNITS), "native manager omitted a unit")
        return result

    def prepare(self, script):
        self.script = safe_path(script)
        self.python = safe_path(Path(sys.executable).resolve(strict=True))
        self.root = safe_path(Path(tempfile.mkdtemp(prefix=".tt-controller-gate-", dir=self.home)))
        self.root_identity = node(self.root)[:4]
        for name in ("config", "data", "receipts", "run", "tools", "tools/__pycache__"):
            (self.root / name).mkdir(mode=0o700)
        private_file(self.root / "deploy.lock", b"")
        self.artifacts[self.root / "deploy.lock"] = node(self.root / "deploy.lock")
        private_file(self.root / "data/preserved.bin", b"synthetic private data\n")
        private_file(self.root / "tools/legacy.py", b"VALUE = 'preserved'\n")
        private_file(self.root / "tools/__pycache__/legacy.fixture.pyc", b"preserved synthetic bytecode\n")
        for _ in UNITS:
            reservation = socket.socket()
            reservation.bind(("127.0.0.1", 0))
            self.reservations.append(reservation)
        self.ports = [item.getsockname()[1] for item in self.reservations]
        settings = dict(zip(("web_port", "api_port", "postgres_port", "redis_port"), self.ports))
        private_file(self.root / "config/settings.json", encoded(settings))
        require(snapshot(self.units_directory, UNITS) == self.foreign, "unit directory changed before fixture creation")
        for unit in UNITS:
            source = ("[Unit]\nDescription=Disposable controller gate " + self.operation + "\n[Service]\n"
                f"Type=simple\nWorkingDirectory={self.root}/data\n"
                f"ExecStart={self.python} -I -B -S {self.script} --worker {self.root} {unit}\n"
                f"Environment={OPT_IN}=1\nMemoryMax=64M\nRuntimeMaxSec=60\nTimeoutStopSec=90\n"
                "KillSignal=SIGINT\nKillMode=control-group\nStandardOutput=null\nStandardError=journal\nUMask=0077\n")
            path = self.units_directory / unit
            private_file(path, source.encode("ascii"))
            self.created[unit] = node(path)

    def original_entries(self):
        require(node(self.units_directory)[:4] == self.directory_identity
                and snapshot(self.units_directory, UNITS) == self.foreign,
                "unit directory changed before native service use")
        require(all(node(self.units_directory / unit) == value for unit, value in self.created.items()),
                "fixture entry changed before native service use")

    def start(self):
        self.original_entries()
        self.command("daemon-reload")
        rows = self.show()
        require(all(row["LoadState"] == "loaded" and not row["DropInPaths"]
            and row["FragmentPath"] == str(self.units_directory / unit)
            and row["MemoryMax"] == str(64 * 1024 * 1024)
            and row["RuntimeMaxUSec"] in ("1min", "60s", "60000000") for unit, row in rows.items()),
            "fixture inherited an unexpected manager override")
        for reservation in self.reservations:
            reservation.close()
        self.original_entries()
        self.command("start", *UNITS)
        rows = self.show()
        failures = []
        for unit, row in rows.items():
            try:
                members = process_snapshot(row["ControlGroup"], timeout=5)
                require(row["ActiveState"] == "active" and len(members) == 1
                        and members[0]["pid"] == int(row["MainPID"]) and members[0]["uid"] == self.uid,
                        "fixture process identity is ambiguous")
                self.tasks[unit] = members[0]
            except (OSError, ValueError, RuntimeError):
                failures.append(unit)
        require(not failures, "fixture workers lacked exact identities: " + ",".join(failures))
        deadline = time.monotonic() + 10
        while not all((self.root / "run" / (unit + ".ready")).exists() for unit in UNITS):
            require(time.monotonic() < deadline, "native fixture readiness expired")
            time.sleep(0.025)
        for unit, port in zip(UNITS, self.ports):
            require(read_json(self.root / "run" / (unit + ".ready")) == {**self.tasks[unit], "port": port},
                    "fixture readiness does not identify the observed process")
            self.artifacts[self.root / "run" / (unit + ".ready")] = node(self.root / "run" / (unit + ".ready"))
        self.original_entries()
        info = self.root.stat()
        target = {"format": "qadra-restore-target", "version": 1, "target_id": str(uuid.uuid4()),
            "root": {"path": str(self.root), "uid": self.uid, "device": info.st_dev, "inode": info.st_ino},
            "environment": {key: value for key, value in self.environment.items() if key != "LC_ALL"},
            "settings": fingerprint(self.root / "config/settings.json"),
            "systemctl": fingerprint(self.controller, 512 * 1024 * 1024), "ports": dict(zip(UNITS, self.ports)),
            "units": {unit: {"fragment": fingerprint(self.units_directory / unit),
                "working_directory": str(self.root / "data"), "control_group": self.tasks[unit]["control_group"]}
                for unit in UNITS}}
        self.target = self.root / "receipts/target.json"
        private_file(self.target, encoded(target))
        self.digest = fingerprint(self.target)["sha256"]
        self.journal = self.root / "maintenance/controller-installation" / self.operation / "journal.json"
        self.protected = self.preserved()
        self.alive()

    def preserved(self):
        return {name: snapshot(self.root / name) for name in ("config", "data", "tools", "receipts")}

    def alive(self):
        for expected in self.tasks.values():
            require(process_snapshot(expected["control_group"], timeout=5) == [expected], "gate changed an existing fixture process")
        require(listening_ports(self.ports, timeout=5) == sorted(self.ports), "fixture listeners changed during gating")

    def owned_entry(self, unit):
        path = self.units_directory / unit
        current = node(path)
        if current == self.created[unit]:
            return current
        require(unit in self.masks and current == self.masks[unit], "cleanup refuses an unpinned or replaced unit entry")
        require(node(self.journal.parent / "slots" / unit) == self.created[unit], "retained original changed before cleanup")
        return current

    def pin_masks(self):
        record = read_json(self.journal)
        require(record["operation_id"] == self.operation and record["target_sha256"] == self.digest,
                "native mask observation belongs to another operation")
        observed = {}
        for unit in UNITS:
            path = self.units_directory / unit
            masked = path.is_symlink()
            mask_path = path if masked else self.journal.parent / "slots" / unit
            original_path = self.journal.parent / "slots" / unit if masked else path
            current, expected = node(mask_path), record["units"][unit]["mask"]
            require(stat.S_ISLNK(current[3]) and current[-1] == "/dev/null"
                and current[:3] == (expected["device"], expected["inode"], self.uid)
                and expected["uid"] == self.uid and node(original_path) == self.created[unit],
                "native gate pair does not preserve its fixture original")
            observed[unit] = current
        self.masks = observed

    def cooperative_receipt(self, unit, task):
        path = self.root / "run" / (unit + ".closed")
        current = node(path)
        require(stat.S_ISREG(current[3]) and stat.S_IMODE(current[3]) == 0o600
                and current[2] == self.uid, "fixture closure receipt metadata differs")
        closed = json.loads(current[-1])
        require(closed in ({**task, "signal": "SIGINT", "status": 0},
                           {**task, "signal": "SIGTERM", "status": 0}),
                "fixture lacks its own cooperative closure receipt")
        self.artifacts[path] = current

    def cleanup(self):
        if self.root is None:
            return
        for reservation in self.reservations:
            reservation.close()
        errors, removed = [], []
        for unit in self.created:
            try:
                require(node(self.units_directory)[:4] == self.directory_identity, "unit directory changed before cleanup")
                owned = self.owned_entry(unit)
                row = self.show()[unit]
                task = self.tasks.get(unit)
                members = process_snapshot(task["control_group"], timeout=5) if task else []
                require((task is not None and members == [task] and int(row["MainPID"]) == task["pid"]
                         and row["ControlGroup"] == task["control_group"] and row["ControlPID"] == "0")
                    or (not members and row["MainPID"] == row["ControlPID"] == "0" and not row["ControlGroup"]),
                    "cleanup refuses an unrecognized running process")
                self.command("stop", unit)
                if task:
                    require(process_snapshot(task["control_group"], timeout=5) == [], "fixture process remains after cleanup stop")
                    final = self.show()[unit]
                    require(final["Result"] == "success" and final["ActiveState"] == "inactive"
                        and final["SubState"] == "dead" and final["MainPID"] == final["ControlPID"] == "0"
                        and not final["ControlGroup"] and final["ExecMainCode"] in ("0", "1")
                        and final["ExecMainStatus"] == "0", "fixture stop has inconsistent final observations")
                    # Mask reload can clear exit metadata; this receipt proves cooperation, not OS exit status.
                    self.cooperative_receipt(unit, task)
                    port = self.ports[UNITS.index(unit)]
                    require(listening_ports([port], timeout=5) == [], "fixture listener remains after cleanup stop")
                path = self.units_directory / unit
                require(node(path) == owned, "unit entry changed during cleanup stop")
                path.unlink()
                removed.append(unit)
            except (OSError, ValueError, RuntimeError, KeyError) as error:
                errors.append(str(error))
        if removed:
            require(node(self.units_directory)[:4] == self.directory_identity, "unit directory changed after cleanup stop")
            sync_directory(self.units_directory)
            self.command("daemon-reload")
        require(snapshot(self.units_directory, UNITS) == self.foreign, "foreign unit inventory changed; preserved fixture evidence")
        if errors or not self.completed:
            raise RuntimeError(f"native fixture evidence retained at {self.root}; cleanup: {errors}")
        require(all(row["LoadState"] == "not-found" and not row["FragmentPath"] for row in self.show().values()),
                "fixture unit names remain installed")
        require(self.preserved() == self.protected, "private fixture inputs changed")
        require(listening_ports(self.ports, timeout=5) == [], "fixture listener survived cleanup")
        self.remove_scratch()

    def remove_scratch(self):
        require(node(self.root)[:4] == self.root_identity, "scratch directory identity changed")
        require(all(node(path) == expected for path, expected in self.artifacts.items()),
                "a pinned fixture artifact changed before cleanup")
        known = {".", "deploy.lock", "config", "config/settings.json", "data", "data/preserved.bin",
                 "receipts", "receipts/target.json", "run", "tools", "tools/legacy.py", "tools/__pycache__",
                 "tools/__pycache__/legacy.fixture.pyc", "maintenance", "maintenance/controller-installation",
                 "maintenance/controller-installation/active.json"}
        operation = "maintenance/controller-installation/" + self.operation
        known.update((operation, operation + "/journal.json", operation + "/slots"))
        for unit in UNITS:
            known.update((operation + "/slots/" + unit, "run/" + unit + ".ready", "run/" + unit + ".closed"))
        observed = snapshot(self.root)
        require(set(observed) == known, "unknown scratch content must be preserved")
        for relative in sorted((name for name in observed if name != "."), key=lambda name: name.count("/"), reverse=True):
            path = self.root / relative
            value = observed[relative]
            if stat.S_ISDIR(value[3]):
                require(node(path)[:4] == value[:4], "scratch directory changed before its removal")
                path.rmdir()
            else:
                require(node(path) == value, "scratch file changed before its removal")
                path.unlink()
        require(node(self.root)[:4] == self.root_identity, "scratch root changed before removal")
        self.root.rmdir()
