"""Published private generations and immutable approval expectations."""
from contextlib import contextmanager, ExitStack
import importlib
import json
import os
from pathlib import Path
import sys
from unittest.mock import patch

from deploy_controller_publication_support import (
    Fixture as PublicationFixture, OPERATION, OTHER_OPERATION, REJECTIONS,
    REPOSITORY, digest, encoded, identity, inventory, journal, module, no_operational_effects,
    put, tree,
)

ENTRIES = ("runtime.py", "release.py", "restore_fence.py", "renew_crl.py")
LAUNCHER = REPOSITORY / "ops/controller_launcher.py"
PYTHON = Path(sys.executable)


def approver():
    return importlib.import_module("controller_approval")


def renderer():
    return importlib.import_module("controller_unit_commands")


@contextmanager
def pure_rendering():
    with no_operational_effects(), ExitStack() as stack:
        for owner, name in ((os, "open"), (os, "stat"), (os, "lstat"),
                            (Path, "read_bytes"), (Path, "read_text"),
                            (Path, "write_bytes"), (Path, "write_text")):
            stack.enter_context(patch.object(
                owner, name, side_effect=AssertionError("unit rendering reached the filesystem")))
        yield


class Fixture(PublicationFixture):
    def __init__(self, case):
        super().__init__(case)
        self.publisher = module()
        self.add_entries("A")
        self.publish(self.publisher)
        self.select_operation(OPERATION)
        self.current_approval = self.root / "maintenance/controllers/approved.json"
        self.marker = self.base / "controller-entered"

    def add_entries(self, generation):
        source = ("import json, os\nfrom pathlib import Path\n"
                  "Path(os.environ['CONTROLLER_APPROVAL_MARKER']).write_text('entered')\n"
                  "print(json.dumps({'generation': " + repr(generation) + "}))\n")
        for entry in ENTRIES:
            put(self.sources / entry, source.encode("ascii"))
        self.new = inventory(self.sources)
        self.manifest["files"] = self.new
        self.save_manifest()

    def select_operation(self, operation_id):
        self.operation_id = operation_id
        self.operation = self.root / "maintenance/controllers" / operation_id
        self.record = self.operation / "journal.json"
        self.exchange = self.operation / "exchange"
        self.operation_approval = self.operation / "approval.json"
        self.publication = json.loads(self.record.read_bytes())
        self.publication_sha256 = digest(self.record.read_bytes())
        self.installed_sha256 = self.publication["installed_sha256"]

    def advance(self):
        old_sources = self.sources
        self.stage = self.base / "next-stage"
        self.sources = self.stage / "sources"
        self.sources.mkdir(parents=True, mode=0o700)
        self.stage.chmod(0o700)
        for path in old_sources.iterdir():
            put(self.sources / path.name, path.read_bytes())
        self.manifest = {**self.manifest, "source_revision": "b" * 40}
        self.add_entries("B")
        self.publish(self.publisher, operation_id=OTHER_OPERATION,
                     expected_previous_sha256=self.installed_sha256)
        self.select_operation(OTHER_OPERATION)

    def expected(self, previous=None):
        record = self.publication
        return {"format": "qadra-controller-approval", "version": 1,
                "operation_id": self.operation_id, "root": record["root"],
                "generation": record["candidate"]["identity"],
                "installed_sha256": record["installed_sha256"],
                "manifest_sha256": record["manifest_sha256"],
                "source_revision": record["source_revision"],
                "publication_sha256": self.publication_sha256,
                "previous_approval_sha256": previous}

    def approve(self, product, previous=None, **overrides):
        values = {"root": self.root, "operation_id": self.operation_id,
                  "expected_publication_sha256": self.publication_sha256,
                  "expected_installed_sha256": self.installed_sha256,
                  "expected_previous_approval_sha256": previous}
        values.update(overrides)
        with no_operational_effects(), patch.object(
                self.publisher, "exchange_directories",
                side_effect=AssertionError("approval performed a source exchange")):
            return product.approve_controllers(**values)

    def assert_approved(self, case, receipt, previous=None):
        expected = self.expected(previous)
        raw = encoded(expected)
        case.assertEqual(self.operation_approval.read_bytes(), raw)
        case.assertEqual(self.current_approval.read_bytes(), raw)
        case.assertEqual(receipt, {"operation_id": self.operation_id,
                                  "approval_sha256": digest(raw),
                                  "installed_sha256": self.installed_sha256})
        for path in (self.operation_approval, self.current_approval):
            row = path.stat()
            case.assertEqual((row.st_uid, row.st_mode & 0o777, row.st_nlink),
                             (os.getuid(), 0o600, 1))
        self.assert_preserved(case)

    def sources_snapshot(self):
        return tree(self.tools), tree(self.exchange), tree(self.stage), self.record.read_bytes()

    def units(self):
        from host import service_units
        return {name + ".service": (content.replace("CPUQuota=200%", "CPUQuota=175%")
                                   + "# preserved private unit tail\n").encode("ascii")
                for name, content in service_units(
                    self.root, "/private/postgres/bin", str(PYTHON), "/private/redis-server").items()}

    def render(self, product, receipt, units, **overrides):
        root_identity = {key: self.publication["root"][key] for key in ("uid", "device", "inode")}
        values = {"root": self.root, "current_units": units,
                  "approval": json.loads(self.current_approval.read_bytes()),
                  "expected_approval_sha256": receipt["approval_sha256"],
                  "expected_root_identity": root_identity,
                  "python_executable": PYTHON, "launcher_path": LAUNCHER}
        values.update(overrides)
        with pure_rendering():
            return product.render_units(**values)


def expected_units(fixture, units, fingerprint):
    result = {}
    prefix = (f"{PYTHON} -I -B -S {LAUNCHER} --root {fixture.root} "
              f"--inventory-sha256 {fingerprint} --entrypoint ")
    for name, value in units.items():
        entry = "restore_fence.py" if name in ("qadra-postgres.service", "qadra-redis.service") else "runtime.py"
        suffix = " --wait-api" if name == "qadra-web.service" else ""
        directive = "ExecStart" if name == "qadra-api.service" else "ExecStartPre"
        before = f"{directive}={PYTHON} {fixture.root}/tools/{entry} {fixture.root}{suffix}\n".encode()
        after = f"{directive}={prefix}{entry} -- {fixture.root}{suffix}\n".encode()
        if value.count(before) != 1:
            raise AssertionError("unit fixture lacks its exact original command")
        result[name] = value.replace(before, after)
    return result
