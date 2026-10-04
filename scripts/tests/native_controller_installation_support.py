"""Owned fixture units, identities and conservative installer acceptance cleanup."""
import hashlib
import os
from pathlib import Path
import socket
import stat
import sys
import tempfile
import time

from native_controller_gate_support import (
    Harness, UNITS, encoded, fingerprint, node, private_file, read_json,
    require, safe_path, snapshot, sync_directory,
)
from restore_quiesce_observe import listening_ports, process_snapshot
import host

ENTRIES = ("runtime.py", "release.py", "restore_fence.py", "renew_crl.py")
SOURCES = (*ENTRIES, "initial.py", "late.py")


def owned(path):
    row = path.lstat()
    return {"uid": row.st_uid, "device": row.st_dev, "inode": row.st_ino}


def digest(value):
    return hashlib.sha256(value).hexdigest()


def inventory(path):
    return {name: {"bytes": item["size"], "sha256": item["sha256"]}
            for name in SOURCES for item in [fingerprint(path / name)]}


class InstallationHarness(Harness):
    def prepare(self, workers):
        self.python = safe_path(Path(sys.executable).resolve(strict=True))
        self.workers = safe_path(workers)
        self.worker_pin = fingerprint(workers)
        self.root = safe_path(Path(tempfile.mkdtemp(prefix=".tt-controller-installation-", dir=self.home)))
        self.root_identity = node(self.root)[:4]
        self.task_history, self.candidate_pins, self.controller_receipts = {}, {}, set()
        self.ports, self.foreign_path = [], None
        self.foreign_before = self.foreign
        for name in ("config", "data", "data/ca", "run", "receipts", "tools", "tools/__pycache__", "fixture-bin",
                     "staged", "staged/sources", "releases", "releases/current"):
            (self.root / name).mkdir(mode=0o700)
        for _ in UNITS:
            reservation = socket.socket()
            reservation.bind(("127.0.0.1", 0))
            self.reservations.append(reservation)
        self.ports = [item.getsockname()[1] for item in self.reservations]
        config = dict(zip(("web_port", "api_port", "postgres_port", "redis_port"), self.ports))
        config.update(admin_password="synthetic-admin", runtime_password="synthetic-runtime",
                      redis_password="synthetic-redis", kek="synthetic-not-a-key")
        private_file(self.root / "config/settings.json", encoded(config))
        private_file(self.root / "deploy.lock", b"")
        private_file(self.root / "data/preserved.bin", b"synthetic private data sentinel\n")
        private_file(self.root / "data/ca/preserved.bin", b"synthetic PKI sentinel, not a key\n")
        self.release = self.root / "releases/current"
        private_file(self.release / "release.json", encoded({
            "version": "native-fixture", "commit": "a" * 40, "schema": "synthetic"}))
        (self.root / "current").symlink_to("releases/current")
        self.generation(self.root / "tools", "A")
        private_file(self.root / "tools/__pycache__/retained.fixture.pyc", b"synthetic retained bytecode\n")
        self.generation(self.root / "staged/sources", "B")
        self.candidate_files = inventory(self.root / "staged/sources")
        self.candidate_sha = digest(encoded(self.candidate_files))
        private_file(self.root / "staged/manifest.json", encoded({
            "format": "qadra-controller-publication", "version": 1,
            "source_revision": "b" * 40, "files": self.candidate_files}))
        for name in ("psql", "redis-cli"):
            value = (f"#!{self.python} -B\nimport runpy, sys\nfrom pathlib import Path\n"
                     f"runpy.run_path({str(workers)!r})['probe'](Path({str(self.root)!r}), {name!r}, sys.argv[1:])\n")
            path = self.root / "fixture-bin" / name
            private_file(path, value.encode("ascii"))
            path.chmod(0o700)
        self.original_units = host.service_units(self.root, self.root / "fixture-bin", self.python)
        for unit in UNITS:
            value = self.original_units[unit.removesuffix(".service")]
            lines = []
            for line in value.splitlines():
                if line.startswith("ExecStart=") and unit != "qadra-api.service":
                    line = f"ExecStart={self.python} -I -B -S {workers} --worker {self.root} {unit}"
                if line.startswith("MemoryMax="):
                    line = "Environment=TT_CONTROLLER_GATE_NATIVE=1\nMemoryMax=64M"
                if line == "Restart=on-failure":
                    line = "Restart=no\nRuntimeMaxSec=120"
                lines.append(line)
            value = ("\n".join(lines) + "\n").encode("ascii")
            path = self.units_directory / unit
            private_file(path, value)
            self.created[unit] = node(path)
        self.original_units = {unit: self.created[unit][-1] for unit in UNITS}
        self.foreign_path = self.units_directory / ("tt-installation-unused-" + self.operation + ".service")
        private_file(self.foreign_path, b"[Service]\nExecStart=/bin/true\n")
        self.foreign_pin = node(self.foreign_path)
        self.foreign = snapshot(self.units_directory, UNITS)
        self.operation_path = self.root / "maintenance/controller-installation" / self.operation
        self.journal = self.operation_path / "journal.json"
        self.protected = self.preserved()

    def generation(self, directory, value):
        for entry in ENTRIES:
            source = ("from initial import VALUE\nfrom pathlib import Path\nimport runpy, sys\n"
                      "if __name__ == '__main__':\n    from late import VALUE as later\n"
                      f"    runpy.run_path({str(self.workers)!r})['controller'](Path(sys.argv[1]), "
                      f"{entry!r}, VALUE, later, sys.argv[2:])\n")
            private_file(directory / entry, source.encode("ascii"))
        for name in ("initial.py", "late.py"):
            private_file(directory / name, ("VALUE = " + repr(value) + "\n").encode("ascii"))

    def preserved(self):
        return {name: snapshot(self.root / name) for name in ("config", "data", "releases", "current")}

    def start(self):
        self.original_entries()
        self.command("daemon-reload")
        require(all(row["LoadState"] == "loaded" and not row["DropInPaths"]
                    and row["FragmentPath"] == str(self.units_directory / unit)
                    and row["MemoryMax"] == str(64 * 1024 * 1024)
                    and row["RuntimeMaxUSec"] in ("2min", "120s", "120000000")
                    for unit, row in self.show().items()), "fixture inherited a manager override")
        for reservation in self.reservations:
            reservation.close()
        self.command("start", *UNITS[2:])
        self.command("start", UNITS[1])
        self.command("start", UNITS[0])
        self.capture_tasks(require_all=True)
        self.initial_tasks = dict(self.tasks)
        require(self.preserved() == self.protected, "fixture startup changed protected inputs")

    def capture_tasks(self, *, require_all=False):
        until = time.monotonic() + 5
        while True:
            rows = self.show()
            missing = False
            for unit, row in rows.items():
                if unit not in self.created:
                    continue
                if row["MainPID"] == "0":
                    missing |= require_all
                    continue
                require(row["ControlPID"] == "0", "fixture still has an unobserved control process")
                members = process_snapshot(row["ControlGroup"], timeout=2)
                require(len(members) == 1 and members[0]["pid"] == int(row["MainPID"])
                        and members[0]["uid"] == self.uid, "fixture process identity differs")
                task = members[0]
                path = self.root / "run" / f"ready-{unit}-{task['pid']}.json"
                if not path.exists():
                    missing = True
                    continue
                require(read_json(path) == {**task, "port": self.ports[UNITS.index(unit)]},
                        "fixture readiness identity differs")
                self.tasks[unit] = task
                self.task_history[(unit, task["pid"])] = task
            if not missing:
                break
            require(time.monotonic() < until, "fixture workers did not report exact readiness")
            time.sleep(0.025)
        if require_all:
            require(listening_ports(self.ports, timeout=2) == sorted(self.ports), "fixture listeners are incomplete")

    def pin_candidates(self):
        self.pin_masks()
        for unit in UNITS:
            self.candidate_pins[unit] = node(self.operation_path / "candidates" / unit)

    def owned_entry(self, unit):
        current = node(self.units_directory / unit)
        allowed = [self.created[unit]]
        if unit in self.masks:
            allowed.append(self.masks[unit])
        if unit in self.candidate_pins:
            allowed.append(self.candidate_pins[unit])
        require(current in allowed, "cleanup refuses an unknown unit entry")
        return current

    def cleanup(self):
        if self.root is None:
            return
        errors = []
        for reservation in self.reservations:
            reservation.close()
        try:
            self.capture_tasks()
        except (OSError, RuntimeError, ValueError):
            errors.append("could not identify every current fixture task")
        for unit in self.created:
            try:
                require(node(self.units_directory)[:4] == self.directory_identity, "unit directory changed")
                current = self.owned_entry(unit)
                row = self.show()[unit]
                require(row["ControlPID"] == "0", "cleanup refuses an unknown control process")
                if row["MainPID"] != "0":
                    task = self.tasks.get(unit)
                    require(task and row["MainPID"] == str(task["pid"])
                            and process_snapshot(task["control_group"], timeout=2) == [task],
                            "cleanup cannot stop an unknown task")
                self.command("stop", unit)
                require(node(self.units_directory / unit) == current, "unit entry changed during cleanup")
                self.units_directory.joinpath(unit).unlink()
            except (OSError, RuntimeError, ValueError):
                errors.append("unit cleanup refused: " + unit)
        if self.created:
            self.command("daemon-reload")
        require(snapshot(self.units_directory, UNITS) == self.foreign, "foreign unit inventory changed")
        if self.foreign_path is not None:
            require(node(self.foreign_path) == self.foreign_pin, "foreign sentinel changed")
            self.foreign_path.unlink()
            sync_directory(self.units_directory)
        require(snapshot(self.units_directory, UNITS) == self.foreign_before, "foreign baseline changed")
        for (unit, pid), task in self.task_history.items():
            require(process_snapshot(task["control_group"], timeout=2) == [], "fixture cgroup remains populated")
            receipt = read_json(self.root / "run" / f"closed-{unit}-{pid}.json")
            require(receipt in ({**task, "signal": "SIGINT", "status": 0},
                                {**task, "signal": "SIGTERM", "status": 0}), "cooperative stop receipt differs")
        require(listening_ports(self.ports, timeout=2) == [], "fixture listener survived cleanup")
        require(all(row["LoadState"] == "not-found" and not row["FragmentPath"]
                    for row in self.show().values()), "fixture unit names remain installed")
        require(not errors and self.completed, f"native fixture retained at {self.root}: {errors}")
        require(self.preserved() == self.protected and fingerprint(self.workers) == self.worker_pin,
                "protected fixture material changed")
        self.remove_owned_scratch()

    def remove_owned_scratch(self):
        observed = snapshot(self.root)
        expected = set(self.initial_snapshot)
        expected.difference_update({name for name in expected if name.startswith("tools/__pycache__")})
        operation = "maintenance/controller-installation/" + self.operation
        publication = "maintenance/controllers/" + self.operation
        expected.update(("maintenance", "maintenance/controller-installation",
                         "maintenance/controllers", "maintenance/controllers/approved.json",
                         "maintenance/controller-installation/active.json", operation, publication,
                         "controller_launcher.py", "receipts/reopen.json"))
        expected.update(operation + "/" + name for name in ("journal.json", "installation.json",
            "stop.json", "closed.json", "reopen.json", "slots", "candidates", "retained-cache"))
        expected.update(publication + "/" + name for name in ("journal.json", "approval.json", "exchange"))
        expected.update(publication + "/exchange/" + name for name in SOURCES)
        expected.update(operation + "/retained-cache/" + name for name in self.cache_names)
        for unit in UNITS:
            expected.update((operation + "/slots/" + unit, operation + "/candidates/" + unit))
        expected.update(self.controller_receipts)
        for unit, pid in self.task_history:
            expected.update((f"run/ready-{unit}-{pid}.json", f"run/closed-{unit}-{pid}.json"))
        require(set(observed) <= expected and node(self.root)[:4] == self.root_identity,
                "unknown scratch content must be retained")
        for relative in sorted((name for name in observed if name != "."),
                               key=lambda name: name.count("/"), reverse=True):
            path, value = self.root / relative, observed[relative]
            require(node(path)[:4] == value[:4] if stat.S_ISDIR(value[3]) else node(path) == value,
                    "scratch object changed before removal")
            path.rmdir() if stat.S_ISDIR(value[3]) else path.unlink()
        self.root.rmdir()
