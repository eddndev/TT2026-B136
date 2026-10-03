"""Install an approved controller generation after exact durable service closure."""
import argparse
import json
import math
from pathlib import Path
import sys
from time import monotonic

from backup_manifest_files import present
from backup_manifest_schema import keys
import controller_gate as gate
import controller_gate_files as gate_files
import controller_installation_files as inputs
import controller_installation_steps as steps
import controller_publication as publication
import controller_publication_files as files
import crl_journal as journal
from restore_commands import run
from restore_quiesce_observe import listening_ports, process_snapshot
import restore_quiesce as quiesce


UNITS = quiesce.UNITS
ERROR = "controller installation failed; reconcile the recorded operation before retrying"
STATES = ("prepared", "stopped", "gated", "published", "closed", "reopening", "installed")


class Installation:
    def __init__(self, root, operation_id, intent_path, digest, deadline):
        self.root, self.operation_id, self.digest, self.deadline = root, operation_id, digest, deadline
        self.intent_path = intent_path
        self.intent = inputs.load(root, operation_id, intent_path, digest)
        self.target_path = Path(self.intent["target"]["path"])
        self.target_digest = self.intent["target"]["sha256"]
        self.target, self.env, self.units = gate.targets.load(root, self.target_path, self.target_digest)
        if files.located(self.units) != self.intent["unit_directory"]:
            raise ValueError("installation unit directory differs from the target")
        self.operation = root / "maintenance/controller-installation" / operation_id
        self.path = self.operation / "installation.json"
        self.closed_path = self.operation / "closed.json"
        self.gate_record = None
        self.record = None
        if present(self.operation):
            files.identity(self.operation)
            self.gate_record = gate_files.read(self.operation / "journal.json")
            if present(self.path):
                self.record = gate_files.read(self.path)
                keys(self.record, ("format", "version", "operation_id", "root", "intent_sha256", "state"))
                expected = self.new_record(self.record["state"])
                if self.record != expected or self.record["state"] not in STATES:
                    raise ValueError("installation journal belongs to another intention")
        self.originals = inputs.originals(root, self.intent, self.target, self.units, self.gate_record)
        inputs.package(root, self.intent)
        inputs.launcher(root, self.intent)
        self.remaining()

    def new_record(self, state):
        return {"format": "qadra-controller-installation", "version": 1,
                "operation_id": self.operation_id, "root": self.intent["root"],
                "intent_sha256": self.digest, "state": state}

    def save(self, state):
        value = self.new_record(state)
        if value != self.record:
            gate_files.save(self.path, value, files.sync_directory, previous=self.record)
            self.record = value
        else:
            gate_files.sync_file(self.path)
            files.sync_directory(self.operation)

    def remaining(self):
        value = self.deadline - monotonic()
        if value <= 0:
            raise RuntimeError("controller installation deadline expired")
        return value

    def live(self):
        self.remaining()
        if inputs.load(self.root, self.operation_id, self.intent_path, self.digest) != self.intent:
            raise ValueError("installation intention changed")
        if gate.targets.load(self.root, self.target_path, self.target_digest) != (self.target, self.env, self.units):
            raise ValueError("installation target context changed")
        self.remaining()

    def command(self, action, units=()):
        available = self.remaining()
        if action == "start" and available < 240:
            raise RuntimeError("installation lacks the configured start budget")
        arguments = [Path(self.target["systemctl"]["path"]), "--user", action, *units]
        if action == "show":
            arguments.append("--property=" + ",".join(gate.targets.PROPERTIES))
        result = run(arguments, env=self.env, text=True, maximum=65536,
                     timeout=min(available, 240) if action == "start" else min(available, 10))
        self.remaining()
        return result.stdout

    def rows(self, orientations, *, complete=False):
        return gate.targets.rows(self.command("show", UNITS), self.target, orientations, complete=complete)

    def absence(self, orientations, *, complete=False):
        self.live()
        rows = self.rows(orientations, complete=complete)
        for unit, row in rows.items():
            if (row["ActiveState"] != "inactive" or row["Result"] != "success"
                    or row["ExecMainCode"] not in ("0", "1") or row["ExecMainStatus"] != "0"):
                raise ValueError("installation closure no longer observes inactive services")
            members = process_snapshot(self.target["units"][unit]["control_group"],
                                       timeout=min(self.remaining(), 10))
            if type(members) is not list or members:
                raise ValueError("installation closure still owns processes")
        ports = listening_ports(list(self.target["ports"].values()), timeout=min(self.remaining(), 10))
        if type(ports) is not list or ports:
            raise ValueError("installation closure still owns listeners")
        self.remaining()

    def orientation(self):
        self.gate_record = gate_files.read(self.operation / "journal.json")
        return gate_files.validate_record(self.gate_record, self.root, self.operation, self.target,
            self.target_digest, self.intent["lock"], self.units, UNITS)

    def preflight_sources(self):
        published = self.root / "maintenance/controllers" / self.operation_id
        publication._parents(self.root)
        if present(published):
            if self.record is None or self.record["state"] in ("prepared", "stopped"):
                raise ValueError("installation cannot adopt an unrelated publication")
            manifest, staging, sources, _ = files.package(Path(self.intent["candidate"]["path"]),
                                                          self.intent["candidate"]["manifest_sha256"])
            publication._resume(self.root, published, self.intent["root"], self.intent["lock"],
                staging, sources, manifest, self.intent["candidate"]["manifest_sha256"],
                self.intent["previous_sources"]["sha256"])
        else:
            inputs.source_inventory(self.root, self.intent)
            inputs.predecessor(self.root, self.intent)
        cache = self.root / "tools/__pycache__"
        retained = self.operation / "retained-cache"
        if present(retained):
            if self.record is None or self.record["state"] in ("prepared", "stopped"):
                raise ValueError("installation retained cache appeared before its durable gate")
            inputs.cache_at(retained, self.intent["cache"])
            inputs.cache_at(cache, None)
        else:
            inputs.cache_at(cache, self.intent["cache"])
        if self.record is None and present(self.operation) and self.gate_record["state"] != "prepared":
            raise ValueError("installation cannot adopt another existing gate")

    def before_stop(self):
        self.live()
        self.preflight_sources()
        inputs.package(self.root, self.intent)
        inputs.launcher(self.root, self.intent)
        if inputs.originals(self.root, self.intent, self.target, self.units) != self.originals:
            raise ValueError("installation original commands changed before stopping")

    def stop(self):
        _, path, record = quiesce.saved(self.root, self.operation_id, self.target_digest, self.target,
                                       path=self.operation / "stop.json")
        if record["state"] == "stopped":
            if set(record["units"]) != set(UNITS):
                raise ValueError("installation lacks complete captured exit evidence")
            self.absence(self.orientation())
            gate_files.sync_file(path)
            files.sync_directory(self.operation)
        else:
            if set(self.orientation().values()) != {"original"}:
                raise ValueError("installation cannot invent stop evidence after masking")
            quiesce._close_locked(self.root, self.target_path, self.target_digest, self.target, self.env,
                min(self.deadline, monotonic() + 240), self.operation, path, record, self.before_stop)
        self.save("stopped")

    def close(self):
        self.preflight_sources()
        if self.record is None:
            self.live()
            gate._close_locked(self.root, self.operation_id, target_path=self.target_path,
                expected_target_sha256=self.target_digest,
                deadline=min(self.deadline, monotonic() + 30), prepare_only=True)
            self.save("prepared")
        if self.record["state"] in ("prepared", "stopped"):
            self.stop()
        else:
            _, _, saved = quiesce.saved(self.root, self.operation_id, self.target_digest, self.target,
                                        path=self.operation / "stop.json")
            if saved["state"] != "stopped" or set(saved["units"]) != set(UNITS):
                raise ValueError("installation lacks durable normal stop evidence")
        self.preflight_sources()
        self.absence(self.orientation())
        gate._close_locked(self.root, self.operation_id, target_path=self.target_path,
            expected_target_sha256=self.target_digest, deadline=min(self.deadline, monotonic() + 30))
        self.absence(self.orientation(), complete=True)
        if self.record["state"] in ("prepared", "stopped"):
            self.save("gated")
        return steps.publish(self)


def install_controllers(root, operation_id, *, intent_path, expected_intent_sha256,
                        reopen_path=None, expected_reopen_sha256=None, timeout=600):
    """Keep closure unless the caller supplies exact external reopening authority."""
    try:
        if type(timeout) not in (int, float) or not math.isfinite(timeout) or not 300 <= timeout <= 900:
            raise ValueError("controller installation timeout is invalid")
        deadline = monotonic() + timeout
        operation_id = publication._identifier(operation_id)
        files.located(root)
        files.identity(root / "deploy.lock", directory=False)
        if Path(__file__).resolve().is_relative_to(root / "tools"):
            raise ValueError("installer must execute outside the generation it replaces")
        if (reopen_path is None) != (expected_reopen_sha256 is None):
            raise ValueError("controller reopening requires both external pins")
        with journal.locked(root):
            current = Installation(root, operation_id, intent_path, expected_intent_sha256, deadline)
            reopening = current.record is not None and current.record["state"] in ("reopening", "installed")
            if reopening and reopen_path is None:
                raise ValueError("controller reopening requires explicit authority on every reentry")
            if reopen_path is not None and not present(current.closed_path):
                raise ValueError("controller reopening requires a previously captured closed receipt")
            if reopening:
                closed = steps.closed(current)
            else:
                if reopen_path is not None:
                    steps.authority(current, steps.closed(current), reopen_path, expected_reopen_sha256, save=False)
                closed = current.close()
            if reopen_path is not None:
                steps.reopen(current, closed, reopen_path, expected_reopen_sha256)
            current.remaining()
            return {"operation_id": operation_id, "intent_sha256": expected_intent_sha256,
                    "state": "installed" if reopen_path is not None else "installed_closed",
                    "closed_sha256": files.read(current.closed_path, 65536)[0]["sha256"]}
    except (OSError, ValueError, RuntimeError, KeyError, TypeError, UnicodeError, RecursionError):
        raise RuntimeError(ERROR) from None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("root", "operation-id", "intent", "intent-sha256"):
        parser.add_argument("--" + name, required=True)
    parser.add_argument("--reopen")
    parser.add_argument("--reopen-sha256")
    parser.add_argument("--timeout", type=float, default=600)
    args = parser.parse_args()
    try:
        receipt = install_controllers(Path(args.root), args.operation_id,
            intent_path=Path(args.intent), expected_intent_sha256=args.intent_sha256,
            reopen_path=None if args.reopen is None else Path(args.reopen),
            expected_reopen_sha256=args.reopen_sha256, timeout=args.timeout)
    except RuntimeError:
        print(ERROR, file=sys.stderr)
        return 1
    print(json.dumps(receipt, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
