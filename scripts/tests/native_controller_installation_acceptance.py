"""Opt-in complete installation against four harmless tt-runner services."""
import json
import os
from pathlib import Path
import re
import sys
import unittest
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parent))
from native_controller_installation_support import (
    InstallationHarness, UNITS, digest, encoded, fingerprint,
    inventory, node, owned, private_file, read_json, require, snapshot,
)
from restore_quiesce_observe import listening_ports, process_snapshot
from restore_commands import run

REPOSITORY = Path(__file__).resolve().parents[2]


def prepare_bootstrap(harness):
    bootstrap = harness.root / "bootstrap"
    bootstrap.mkdir(mode=0o700)
    tools = bootstrap / "tools"
    tools.mkdir(mode=0o700)
    sources = sorted((REPOSITORY / "ops/deploy").glob("*.py"))
    require(0 < len(sources) <= 64 and any(path.name == "controller_installation.py" for path in sources),
            "bootstrap requires the actual completed installation sources")
    files = {}
    for source in sources:
        require(re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*\.py", source.name), "invalid bootstrap source name")
        before = fingerprint(source)
        value = source.read_bytes()
        require(len(value) == before["size"] and digest(value) == before["sha256"]
                and fingerprint(source) == before, "bootstrap source changed while copying")
        private_file(tools / source.name, value)
        files[source.name] = {"bytes": len(value), "sha256": digest(value)}
    return {"root": bootstrap, "sha256": digest(encoded(files)), "snapshot": snapshot(bootstrap)}


def prepare_intent(harness):
    root = harness.root
    target = {"format": "qadra-restore-target", "version": 1, "target_id": str(uuid.uuid4()),
        "root": {"path": str(root), **owned(root)},
        "environment": {key: value for key, value in harness.environment.items() if key != "LC_ALL"},
        "settings": fingerprint(root / "config/settings.json"),
        "systemctl": fingerprint(harness.controller, 512 * 1024 * 1024),
        "ports": dict(zip(UNITS, harness.ports)),
        "units": {unit: {"fragment": fingerprint(harness.units_directory / unit),
            "working_directory": str(root / "data"),
            "control_group": harness.initial_tasks[unit]["control_group"]} for unit in UNITS}}
    harness.target = root / "receipts/target.json"
    private_file(harness.target, encoded(target))
    harness.digest = fingerprint(harness.target)["sha256"]
    launcher_source = root / "launcher-source.py"
    private_file(launcher_source, (REPOSITORY / "ops/controller_launcher.py").read_bytes())
    cache = root / "tools/__pycache__"
    harness.cache_names = sorted(path.name for path in cache.iterdir())
    harness.cache_before = snapshot(cache)
    harness.before_files = inventory(root / "tools")
    harness.intent = {"format": "qadra-controller-installation-intent", "version": 1,
        "operation_id": harness.operation, "root": target["root"],
        "lock": owned(root / "deploy.lock"),
        "unit_directory": {"path": str(harness.units_directory), **owned(harness.units_directory)},
        "target": fingerprint(harness.target),
        "previous_sources": {"identity": owned(root / "tools"),
                             "sha256": digest(encoded(harness.before_files))},
        "cache": {"identity": owned(cache), "files": {name: {
            "identity": owned(cache / name), "bytes": (cache / name).stat().st_size,
            "sha256": fingerprint(cache / name)["sha256"]} for name in harness.cache_names}},
        "candidate": {"path": str(root / "staged"),
            "manifest_sha256": fingerprint(root / "staged/manifest.json")["sha256"],
            "installed_sha256": harness.candidate_sha},
        "previous_approval_sha256": None,
        "launcher": {"source": fingerprint(launcher_source), "destination": str(root / "controller_launcher.py")},
        "python": fingerprint(harness.python, 512 * 1024 * 1024), "legacy_absent_prestarts": [],
        "release": {"link": os.readlink(root / "current"),
                    "metadata": fingerprint(harness.release / "release.json")}}
    harness.intent_path = root / "receipts/installation.json"
    private_file(harness.intent_path, encoded(harness.intent))
    harness.intent_sha = fingerprint(harness.intent_path)["sha256"]
    harness.initial_snapshot = snapshot(root)


def invoke(product, harness, reopen=None, digest_override=None):
    require(snapshot(product["root"]) == product["snapshot"], "bootstrap code snapshot changed")
    launcher = harness.root / "launcher-source.py"
    require(fingerprint(launcher) == harness.intent["launcher"]["source"], "bootstrap launcher pin changed")
    arguments = [harness.python, "-I", "-B", "-S", launcher,
        "--root", product["root"], "--inventory-sha256", product["sha256"],
        "--entrypoint", "controller_installation.py", "--",
        "--root", harness.root, "--operation-id", harness.operation,
        "--intent", harness.intent_path, "--intent-sha256", harness.intent_sha, "--timeout", "600"]
    if reopen is not None:
        arguments.extend(("--reopen", reopen, "--reopen-sha256",
                          digest_override or fingerprint(reopen)["sha256"]))
    environment = {**harness.environment, "TT_CONTROLLER_GATE_NATIVE": "1",
                   "PATH": str(harness.root / "fixture-bin") + os.pathsep + os.defpath}
    result = run(arguments, env=environment, cwd=product["root"], timeout=300,
                 maximum=65536, text=True)
    require(snapshot(product["root"]) == product["snapshot"], "bootstrap execution changed its code snapshot")
    require(len(result.stdout.splitlines()) == 1, "installation CLI did not return exactly one JSON receipt")
    receipt = json.loads(result.stdout)
    require(type(receipt) is dict, "installation CLI receipt is not an object")
    return receipt


def generation_records(harness, generation):
    records = {}
    for path in (harness.root / "run").glob("generation-" + generation + "-*.json"):
        value = read_json(path)
        unit = value["control_group"].rsplit("/", 1)[-1]
        require(unit in UNITS and unit not in records and value["uid"] == harness.uid
                and value["initial"] == value["late"] == generation, "generation receipt is ambiguous")
        expected_entry = "runtime.py" if unit in UNITS[:2] else "restore_fence.py"
        expected_args = ["--wait-api"] if unit == UNITS[0] else []
        require(value["entry"] == expected_entry and value["arguments"] == expected_args,
                "generation entrypoint differs")
        require(path.name == f"generation-{generation}-{unit}-{value['pid']}.json",
                "generation receipt filename differs")
        harness.controller_receipts.add(str(path.relative_to(harness.root)))
        records[unit] = value
    require(set(records) == set(UNITS), "four actual controller invocations are required")
    return records


def expected_candidate(harness, unit):
    entry = "restore_fence.py" if unit in UNITS[2:] else "runtime.py"
    directive = "ExecStart" if unit == UNITS[1] else "ExecStartPre"
    suffix = " --wait-api" if unit == UNITS[0] else ""
    original = f"{directive}={harness.python} {harness.root}/tools/{entry} {harness.root}{suffix}\n".encode()
    replacement = (f"{directive}={harness.python} -I -B -S {harness.root}/controller_launcher.py "
        f"--root {harness.root} --inventory-sha256 {harness.candidate_sha} "
        f"--entrypoint {entry} -- {harness.root}{suffix}\n").encode()
    raw = harness.original_units[unit]
    require(raw.count(original) == 1, "fixture did not preserve the literal legacy command")
    return raw.replace(original, replacement), directive, replacement.decode().strip().split("=", 1)[1]


@unittest.skipUnless(os.environ.get("TT_CONTROLLER_GATE_NATIVE") == "1",
                     "explicit disposable installer acceptance is required")
class NativeControllerInstallation(unittest.TestCase):
    def test_real_stop_gate_publication_authority_and_readiness_preserve_foreign_state(self):
        harness = InstallationHarness()
        previous_path = os.environ.get("PATH")
        try:
            harness.prepare(Path(__file__).with_name("native_controller_installation_workers.py").resolve())
            product = prepare_bootstrap(harness)
            os.environ["PATH"] = str(harness.root / "fixture-bin") + os.pathsep + os.defpath
            harness.start()
            before_generations = generation_records(harness, "A")
            self.assertEqual(before_generations[UNITS[1]]["pid"], harness.initial_tasks[UNITS[1]]["pid"])
            prepare_intent(harness)
            closed = invoke(product, harness)
            self.assertEqual(closed, {"operation_id": harness.operation,
                "intent_sha256": harness.intent_sha, "state": "installed_closed",
                "closed_sha256": fingerprint(harness.operation_path / "closed.json")["sha256"]})
            harness.pin_candidates()
            self.assertTrue(all(row["LoadState"] == row["UnitFileState"] == "masked"
                                and row["MainPID"] == row["ControlPID"] == "0"
                                and row["NeedDaemonReload"] == "no" for row in harness.show().values()))
            self.assertEqual(listening_ports(harness.ports, timeout=2), [])
            for task in harness.initial_tasks.values():
                self.assertEqual(process_snapshot(task["control_group"], timeout=2), [])
            self.assertEqual(inventory(harness.root / "tools"), harness.candidate_files)
            self.assertEqual(snapshot(harness.operation_path / "retained-cache"), harness.cache_before)
            self.assertFalse((harness.root / "maintenance/restore").exists())
            self.assertEqual(harness.preserved(), harness.protected)
            self.assertEqual((harness.root / "controller_launcher.py").read_bytes(),
                             (REPOSITORY / "ops/controller_launcher.py").read_bytes())
            record = read_json(harness.operation_path / "closed.json")
            self.assertEqual(record["installed_sha256"], harness.candidate_sha)
            self.assertEqual(record["stop_sha256"], fingerprint(harness.operation_path / "stop.json")["sha256"])
            self.assertEqual(record["approval_sha256"],
                             fingerprint(harness.root / "maintenance/controllers/approved.json")["sha256"])
            for unit in UNITS:
                self.assertEqual(node(harness.operation_path / "slots" / unit), harness.created[unit])
                self.assertEqual((harness.operation_path / "candidates" / unit).read_bytes(),
                                 expected_candidate(harness, unit)[0])
            closed_snapshot = snapshot(harness.operation_path)
            self.assertEqual(invoke(product, harness), closed)
            self.assertEqual(snapshot(harness.operation_path), closed_snapshot)
            self.assertEqual(list((harness.root / "run").glob("generation-B-*.json")), [])
            authority = harness.root / "receipts/reopen.json"
            private_file(authority, encoded({"format": "qadra-controller-reopen", "version": 1,
                "operation_id": harness.operation, "root": harness.intent["root"],
                "intent_sha256": harness.intent_sha, "closed_sha256": closed["closed_sha256"]}))
            with self.assertRaises(RuntimeError):
                invoke(product, harness, authority, "0" * 64)
            self.assertEqual(snapshot(harness.operation_path), closed_snapshot)
            self.assertEqual(listening_ports(harness.ports, timeout=2), [])
            installed = invoke(product, harness, authority)
            self.assertEqual(installed, {**closed, "state": "installed"})
            harness.capture_tasks(require_all=True)
            after_generations = generation_records(harness, "B")
            self.assertEqual(after_generations[UNITS[1]]["pid"], harness.tasks[UNITS[1]]["pid"])
            for unit, task in harness.tasks.items():
                self.assertNotEqual(task, harness.initial_tasks[unit])
                candidate, directive, command = expected_candidate(harness, unit)
                self.assertEqual((harness.units_directory / unit).read_bytes(), candidate)
                raw = harness.command("show", unit, "--property=" + directive)
                found = re.findall(r"argv\[\]=(.*?) ;", raw)
                self.assertEqual(found, [command])
            self.assertEqual(harness.preserved(), harness.protected)
            self.assertFalse((harness.root / "tools/__pycache__").exists())
            pids = dict(harness.tasks)
            after = snapshot(harness.operation_path)
            self.assertEqual(invoke(product, harness, authority), installed)
            harness.capture_tasks(require_all=True)
            self.assertEqual(harness.tasks, pids)
            self.assertEqual(snapshot(harness.operation_path), after)
            self.assertEqual(generation_records(harness, "A"), before_generations)
            self.assertEqual(generation_records(harness, "B"), after_generations)
            harness.completed = True
        finally:
            try:
                harness.cleanup()
            finally:
                if previous_path is None:
                    os.environ.pop("PATH", None)
                else:
                    os.environ["PATH"] = previous_path


if __name__ == "__main__":
    os.umask(0o077)
    unittest.main()
