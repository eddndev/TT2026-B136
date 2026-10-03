"""Innocuous complete generations composed with the real installation modules."""
from contextlib import ExitStack, contextmanager
import importlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
from unittest.mock import patch

from deploy_controller_installation_edges import InstallationEdges
from deploy_controller_publication_support import REPOSITORY, digest, encoded, inventory, put, tree
from deploy_restore_quiesce_refs_support import ReferenceFixture
from deploy_restore_quiesce_support import OPERATION, UNITS, fingerprint, host, journal


ENTRIES = ("runtime.py", "release.py", "restore_fence.py", "renew_crl.py")
BOOTSTRAP = "30000000-0000-4000-8000-000000000003"


def owned(path):
    info = path.lstat()
    return {"uid": info.st_uid, "device": info.st_dev, "inode": info.st_ino}


class InstallationFixture(InstallationEdges, ReferenceFixture):
    def setUp(self):
        super().setUp()
        self.systemctl.rename(self.base / "systemctl")
        self.systemctl = self.base / "systemctl"
        self.target["systemctl"] = fingerprint(self.systemctl)
        self.python = Path(sys.executable).resolve(strict=True)
        generated = host.service_units(self.root, self.base / "postgres", self.python, self.base / "redis")
        self.original_units = {}
        for unit in UNITS:
            raw = generated[unit.removesuffix(".service")].replace("CPUQuota=200%", "CPUQuota=175%")
            if unit in UNITS[2:]:
                raw = "\n".join(line for line in raw.split("\n") if not line.startswith("ExecStartPre="))
            raw = (raw + "# preserved private unit tail\n").encode("ascii")
            put(self.units_directory / unit, raw)
            self.original_units[unit] = raw
            self.target["units"][unit]["fragment"] = fingerprint(self.units_directory / unit)
            self.units[unit]["UnitFileState"] = "enabled"
        self.pin_target()
        put(self.root / "deploy.lock", b"")
        put(self.units_directory / "foreign.service", b"[Service]\nExecStart=/bin/true\n")
        put(self.root / "data/pki/private/synthetic.key", b"never replace private fixture material\n")
        self.release = self.root / "releases/current"
        put(self.release / "release.json", encoded({"version": "v1.0.0", "commit": "a" * 40, "schema": "fixed"}))
        (self.root / "current").symlink_to("releases/current")
        self.generation(self.root / "tools", "A")
        self.previous_files = inventory(self.root / "tools")
        self.cache = self.root / "tools/__pycache__"
        put(self.cache / "runtime.fixture.pyc", b"retained legacy bytecode\n")
        self.cache_before = tree(self.cache)
        self.stage = self.base / "candidate"
        self.generation(self.stage / "sources", "B")
        self.candidate_files = inventory(self.stage / "sources")
        self.candidate_sha = digest(encoded(self.candidate_files))
        put(self.stage / "manifest.json", encoded({"format": "qadra-controller-publication", "version": 1,
            "source_revision": "b" * 40, "files": self.candidate_files}))
        self.launcher_source = self.base / "controller_launcher.py"
        put(self.launcher_source, (REPOSITORY / "ops/controller_launcher.py").read_bytes())
        self.launcher = self.root / "controller_launcher.py"
        self.operation = self.root / "maintenance/controller-installation" / OPERATION
        self.stop_record = self.operation / "stop.json"
        self.intent_record = self.operation / "installation.json"
        self.closed_record = self.operation / "closed.json"
        self.reopen_record = self.operation / "reopen.json"
        self.retained_cache = self.operation / "retained-cache"
        self.candidates = self.operation / "candidates"
        self.record = self.stop_record
        self.publication = self.root / "maintenance/controllers" / OPERATION
        self.current_approval = self.publication.parent / "approved.json"
        self.intent_path = self.root / "receipts/controller-installation.json"
        self.authority_path = self.root / "receipts/controller-reopen.json"
        self.intent = {"format": "qadra-controller-installation-intent", "version": 1,
            "operation_id": OPERATION, "root": {"path": str(self.root), **owned(self.root)},
            "lock": owned(self.root / "deploy.lock"),
            "unit_directory": {"path": str(self.units_directory), **owned(self.units_directory)},
            "target": fingerprint(self.target_path),
            "previous_sources": {"identity": owned(self.root / "tools"), "sha256": digest(encoded(self.previous_files))},
            "cache": {"identity": owned(self.cache), "files": {path.name: {
                "identity": owned(path), "bytes": path.stat().st_size, "sha256": digest(path.read_bytes())}
                for path in self.cache.iterdir()}},
            "candidate": {"path": str(self.stage), "manifest_sha256": digest((self.stage / "manifest.json").read_bytes()),
                          "installed_sha256": self.candidate_sha},
            "previous_approval_sha256": None,
            "launcher": {"source": fingerprint(self.launcher_source), "destination": str(self.launcher)},
            "python": fingerprint(self.python), "legacy_absent_prestarts": list(UNITS[2:]),
            "release": {"link": "releases/current", "metadata": fingerprint(self.release / "release.json")}}
        self.pin_intent()
        self.protected = self.preserved()
        self.fault, self.fault_reached, self.stop_durable = None, False, False
        self.fsynced = set()
        self.timeout = 600

    def generation(self, directory, value):
        source = ("import json\nfrom initial import VALUE\n"
                  "def main():\n    from late import VALUE as later\n"
                  "    print(json.dumps({'initial': VALUE, 'late': later}))\nmain()\n")
        for entry in ENTRIES:
            put(directory / entry, source.encode("ascii"))
        for name in ("initial.py", "late.py"):
            put(directory / name, ("VALUE = " + repr(value) + "\n").encode("ascii"))

    def pin_intent(self):
        put(self.intent_path, encoded(self.intent))
        self.intent_sha = digest(self.intent_path.read_bytes())

    def python_inventory(self, directory):
        return {path.name: {"bytes": path.stat().st_size, "sha256": digest(path.read_bytes())}
                for path in sorted(directory.glob("*.py"))}

    def preserved(self):
        return {str(path): tree(path) for path in (self.root / "config", self.root / "data",
                self.root / "releases", self.root / "current", self.units_directory / "foreign.service")}

    @contextmanager
    def controller(self):
        product = importlib.import_module("controller_installation")
        quiesce = importlib.import_module("restore_quiesce")
        gate = importlib.import_module("controller_gate")
        gate_files = importlib.import_module("controller_gate_files")
        publisher = importlib.import_module("controller_publication")
        runtime = importlib.import_module("runtime")
        command = importlib.import_module("restore_commands")
        self.real_locked, self.real_fsync = journal.locked, os.fsync
        self.real_rename, self.real_source_exchange = gate_files._rename, publisher.exchange_directories
        self.real_replace = os.replace
        with ExitStack() as stack:
            stack.enter_context(patch.dict(os.environ, {**self.environment, "UNRELATED_SECRET": self.secret}, clear=True))
            for owner, name, effect in (
                (journal, "locked", self.lock), (os, "fsync", self.sync_file), (os, "replace", self.replace),
                (gate_files, "_rename", self.rename), (publisher, "exchange_directories", self.exchange_sources),
                (quiesce, "retain_units", self.retain_units), (runtime.Runtime, "check", self.ready),
            ):
                stack.enter_context(patch.object(owner, name, autospec=True, side_effect=effect))
            for owner in (product, quiesce, gate, command):
                stack.enter_context(patch.object(owner, "run", side_effect=self.execute_installation, create=True))
            for owner in (product, quiesce, gate):
                stack.enter_context(patch.object(owner, "monotonic", side_effect=lambda: self.now, create=True))
                stack.enter_context(patch.object(owner, "process_snapshot", side_effect=self.processes, create=True))
                stack.enter_context(patch.object(owner, "listening_ports", side_effect=self.listeners, create=True))
            for owner, name in ((host, "prepare"), (host, "start_databases"), (runtime.Runtime, "start"),
                                (runtime.Runtime, "initialize"), (runtime.Runtime, "backup"),
                                (subprocess, "Popen"), (os, "kill"), (os, "killpg")):
                stack.enter_context(patch.object(owner, name, side_effect=AssertionError("unexpected operational edge")))
            yield product

    def invoke(self, product, *, operation=OPERATION, reopen=False, **overrides):
        arguments = {"intent_path": self.intent_path, "expected_intent_sha256": self.intent_sha,
                     "timeout": self.timeout}
        if reopen:
            arguments.update(reopen_path=self.authority_path,
                             expected_reopen_sha256=digest(self.authority_path.read_bytes()))
        arguments.update(overrides)
        return product.install_controllers(self.root, operation, **arguments)

    def authorize(self, receipt):
        value = {"format": "qadra-controller-reopen", "version": 1, "operation_id": OPERATION,
                 "root": self.intent["root"], "intent_sha256": self.intent_sha,
                 "closed_sha256": receipt["closed_sha256"]}
        put(self.authority_path, encoded(value))

    def expected_unit(self, unit):
        raw = self.original_units[unit]
        entry = "restore_fence.py" if unit in UNITS[2:] else "runtime.py"
        directive = "ExecStart" if unit == UNITS[1] else "ExecStartPre"
        suffix = " --wait-api" if unit == UNITS[0] else ""
        after = (f"{directive}={self.python} -I -B -S {self.launcher} --root {self.root} "
                 f"--inventory-sha256 {self.candidate_sha} --entrypoint {entry} -- {self.root}{suffix}\n").encode()
        if unit in UNITS[2:]:
            return raw.replace(b"MemoryMax=", after + b"MemoryMax=", 1)
        before = f"{directive}={self.python} {self.root}/tools/{entry} {self.root}{suffix}\n".encode()
        self.assertEqual(raw.count(before), 1)
        return raw.replace(before, after)

    def assert_closed(self, receipt):
        self.assertEqual(receipt, {"operation_id": OPERATION, "intent_sha256": self.intent_sha,
            "state": "installed_closed", "closed_sha256": digest(self.closed_record.read_bytes())})
        self.assertEqual(inventory(self.root / "tools"), self.candidate_files)
        self.assertEqual(tree(self.retained_cache), self.cache_before)
        self.assertEqual(self.preserved(), self.protected)
        self.assertFalse((self.root / "maintenance/restore").exists())
        self.assertFalse(self.references_active)
        self.assertEqual(self.launcher.read_bytes(), self.launcher_source.read_bytes())
        for unit in UNITS:
            self.assertEqual(os.readlink(self.units_directory / unit), "/dev/null")
            self.assertEqual((self.operation / "slots" / unit).read_bytes(), self.original_units[unit])
            self.assertEqual((self.candidates / unit).read_bytes(), self.expected_unit(unit))
        closed = json.loads(self.closed_record.read_bytes())
        self.assertEqual(closed["approval_sha256"], digest(self.current_approval.read_bytes()))
        self.assertEqual(closed["installed_sha256"], self.candidate_sha)
        self.assertEqual(json.loads((self.publication / "journal.json").read_bytes())["state"], "published")

    def bootstrap_approval(self):
        publisher = importlib.import_module("controller_publication")
        approver = importlib.import_module("controller_approval")
        self.cache.rename(self.base / "cache-temporary")
        stage = self.base / "baseline"
        self.generation(stage / "sources", "A")
        put(stage / "manifest.json", encoded({"format": "qadra-controller-publication", "version": 1,
            "source_revision": "a" * 40, "files": inventory(stage / "sources")}))
        publisher.publish_controllers(self.root, BOOTSTRAP, stage,
            expected_manifest_sha256=digest((stage / "manifest.json").read_bytes()),
            expected_previous_sha256=self.intent["previous_sources"]["sha256"])
        record = self.root / "maintenance/controllers" / BOOTSTRAP / "journal.json"
        receipt = approver.approve_controllers(self.root, BOOTSTRAP,
            expected_publication_sha256=digest(record.read_bytes()),
            expected_installed_sha256=self.intent["previous_sources"]["sha256"],
            expected_previous_approval_sha256=None)
        (self.base / "cache-temporary").rename(self.cache)
        self.intent["previous_sources"]["identity"] = owned(self.root / "tools")
        self.intent["previous_approval_sha256"] = receipt["approval_sha256"]
        self.pin_intent()

    def run_candidate(self):
        arguments = shlex.split(self.expected_unit(UNITS[1]).decode().split("ExecStart=", 1)[1].splitlines()[0])
        result = subprocess.run(arguments, check=True, capture_output=True, text=True, timeout=10,
                                env={"PATH": os.defpath})
        self.assertEqual(json.loads(result.stdout), {"initial": "B", "late": "B"})
