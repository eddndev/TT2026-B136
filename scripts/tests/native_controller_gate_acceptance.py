"""Opt-in persistent entry gates on four harmless units of the tt-runner manager."""
import json
import os
from pathlib import Path
import pwd
import signal
import socket
import subprocess
import sys
import threading
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
from native_controller_gate_support import (
    Harness, OPT_IN, UNITS, encoded, node, read_json, require, safe_path, sync_directory,
)
import controller_gate as gate
from controller_gate_files import save
from restore_quiesce_observe import membership, start_time


def worker(root, unit):
    require(os.environ.get(OPT_IN) == "1" and pwd.getpwuid(os.getuid()).pw_name == "tt-runner",
            "fixture worker requires the explicit disposable account")
    safe_path(root)
    require(root.name.startswith(".tt-controller-gate-") and root.stat().st_uid == os.getuid()
            and unit in UNITS, "fixture worker target is invalid")
    stopped, received = threading.Event(), []

    def stop(number, _frame):
        received.append(signal.Signals(number).name)
        stopped.set()

    signal.signal(signal.SIGINT, stop)
    signal.signal(signal.SIGTERM, stop)
    settings = read_json(root / "config/settings.json")
    key = dict(zip(UNITS, ("web_port", "api_port", "postgres_port", "redis_port")))[unit]
    pid = os.getpid()
    group = membership(Path(f"/proc/{pid}/cgroup").read_bytes())
    require(group.endswith("/" + unit), "fixture worker is outside its owned unit group")
    ticks = start_time(Path(f"/proc/{pid}/stat").read_bytes(), pid)
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", settings[key]))
        listener.listen(1)
        save(root / "run" / (unit + ".ready"), {"pid": pid, "uid": os.getuid(),
            "start_ticks": ticks, "control_group": group, "port": settings[key]}, sync_directory)
        require(stopped.wait(50), "fixture worker exceeded its lifetime")
    save(root / "run" / (unit + ".closed"), {"signal": received[0], "status": 0,
        "pid": pid, "uid": os.getuid(), "start_ticks": ticks, "control_group": group}, sync_directory)


def close_child(root, operation, target, digest, mode):
    require(os.environ.get(OPT_IN) == "1" and pwd.getpwuid(os.getuid()).pw_name == "tt-runner",
            "fixture controller requires the explicit disposable account")
    require(mode in ("partial", "lost-ack", "complete"), "fixture controller mode is invalid")
    require(root.name.startswith(".tt-controller-gate-") and target == root / "receipts/target.json",
            "fixture controller target is invalid")
    exchanges, acknowledged = 0, False
    exchange, command = gate._exchange, gate.run

    def partial(fragment, slot):
        nonlocal exchanges
        exchange(fragment, slot)
        exchanges += 1
        if exchanges == 2:
            os.write(1, b"partial-mask\n")
            os._exit(86)

    def lost_ack(arguments, **kwargs):
        nonlocal acknowledged
        result = command(arguments, **kwargs)
        if str(arguments[2]) == "daemon-reload":
            acknowledged = True
            raise RuntimeError("intentional native reload acknowledgement loss")
        return result

    if mode == "partial":
        gate._exchange = partial
    if mode == "lost-ack":
        gate.run = lost_ack
    try:
        receipt = gate.close_entries(root, operation, target_path=target,
                                     expected_target_sha256=digest, timeout=8)
    except RuntimeError:
        if mode == "lost-ack" and acknowledged:
            print("acknowledgement-lost", flush=True)
            return
        raise
    require(mode == "complete", "fixture missed its requested interruption")
    print(encoded(receipt).decode("ascii"), end="", flush=True)


def invoke(harness, mode):
    arguments = [harness.python, "-I", "-B", "-S", "-u", harness.script, "--close",
                 harness.root, harness.operation, harness.target, harness.digest, mode]
    process = subprocess.Popen([str(value) for value in arguments], env={**harness.environment, OPT_IN: "1"},
                               stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               text=True, encoding="ascii", start_new_session=True)
    try:
        output, error = process.communicate(timeout=12)
        require(len(output) <= 65536 and len(error) <= 65536, "native controller output exceeds its boundary")
        require(error == "", "native controller child failed: " + error)
        require(process.returncode == (86 if mode == "partial" else 0), "native controller child exit differs")
        return output
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3)
        for stream in (process.stdout, process.stderr):
            stream.close()


@unittest.skipUnless(os.environ.get(OPT_IN) == "1", "explicit disposable native gate acceptance is required")
class NativeControllerGate(unittest.TestCase):
    def test_persistent_masks_partial_death_reload_uncertainty_and_fresh_reentry_preserve_running_workers(self):
        harness = Harness()
        try:
            harness.prepare(Path(__file__).resolve())
            harness.start()
            self.assertEqual(invoke(harness, "partial"), "partial-mask\n")
            self.assertEqual(read_json(harness.journal)["state"], "masking")
            harness.pin_masks()
            masked = {unit for unit in UNITS if (harness.units_directory / unit).is_symlink()}
            self.assertEqual(len(masked), 2)
            for unit in UNITS:
                original = harness.journal.parent / "slots" / unit if unit in masked else harness.units_directory / unit
                self.assertEqual(node(original), harness.created[unit])
            first_masks = {unit: node(harness.units_directory / unit) for unit in masked}
            harness.alive()
            self.assertEqual(invoke(harness, "lost-ack"), "acknowledgement-lost\n")
            self.assertEqual(read_json(harness.journal)["state"], "reload_pending")
            loaded = harness.show()
            self.assertTrue(all(row["LoadState"] == row["UnitFileState"] == "masked"
                                and row["NeedDaemonReload"] == "no" for row in loaded.values()))
            for unit, value in first_masks.items():
                self.assertEqual(node(harness.units_directory / unit), value)
            expected = {"operation_id": harness.operation, "target_sha256": harness.digest, "state": "gated"}
            self.assertEqual(json.loads(invoke(harness, "complete")), expected)
            self.assertEqual(read_json(harness.journal)["state"], "gated")
            masks = {unit: harness.owned_entry(unit) for unit in UNITS}
            harness.alive()
            self.assertEqual(json.loads(invoke(harness, "complete")), expected)
            self.assertEqual({unit: harness.owned_entry(unit) for unit in UNITS}, masks)
            self.assertTrue(all(row["LoadState"] == row["UnitFileState"] == "masked"
                                and row["NeedDaemonReload"] == "no" for row in harness.show().values()))
            harness.alive()
            self.assertEqual(harness.preserved(), harness.protected)
            marker = harness.journal.parent.parent / "active.json"
            self.assertEqual(read_json(marker), {"operation_id": harness.operation, "target_sha256": harness.digest})
            for path in (marker, harness.journal):
                harness.artifacts[path] = node(path)
            harness.completed = True
        finally:
            harness.cleanup()


if __name__ == "__main__":
    os.umask(0o077)
    if len(sys.argv) == 4 and sys.argv[1] == "--worker":
        worker(Path(sys.argv[2]), sys.argv[3])
    elif len(sys.argv) == 7 and sys.argv[1] == "--close":
        close_child(Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]), sys.argv[5], sys.argv[6])
    else:
        unittest.main()
