"""Check cancellation and failure cleanup for disposable CI services."""

from contextlib import contextmanager
import importlib.util
from pathlib import Path
import signal
import tempfile
import unittest
from unittest.mock import MagicMock, Mock, patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "ci_native_postgres", ROOT / "scripts/ci_native_postgres.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


@contextmanager
def startup_backend(effective_locks):
    events = []

    def execute(command, **_kwargs):
        events.append(("run", command))
        return Mock(returncode=0)

    def output(command, **_kwargs):
        events.append(("output", command))
        if command == ["postgres", "--version"]:
            return "postgres (PostgreSQL) 16.10\n"
        if command[0] == "systemctl" and command[-3:] == ["-p", "MemoryMax", "--value"]:
            return str(MODULE.MEMORY_BYTES) + "\n"
        if command[0] == "psql" and command[-2:] == ["-c", "SHOW max_locks_per_transaction"]:
            return str(effective_locks) + "\n"
        raise AssertionError(f"unexpected startup command: {command[0]}")

    socket_context = MagicMock()
    socket_context.__enter__.return_value.getsockname.return_value = ("127.0.0.1", 54321)
    with tempfile.TemporaryDirectory() as temporary, patch.dict(
        MODULE.os.environ, {"RUNNER_TEMP": temporary, "GITHUB_ACTIONS": "false"}
    ), patch.object(MODULE.shutil, "which", side_effect=lambda name: f"/usr/bin/{name}"), patch.object(
        MODULE.subprocess, "run", side_effect=execute
    ), patch.object(MODULE.subprocess, "check_output", side_effect=output), patch.object(
        MODULE.socket, "socket", return_value=socket_context
    ), patch("builtins.print"):
        yield MODULE.Backend(), events


class TestNativePostgres(unittest.TestCase):
    def test_start_configures_and_verifies_shared_lock_capacity(self):
        with startup_backend(256) as (backend, events):
            environment = backend.start()
        commands = [command for operation, command in events if operation == "run"]
        start = next(command for command in commands if command[0] == "systemd-run")
        self.assertIn(("-c", "max_locks_per_transaction=256"), list(zip(start, start[1:])))
        checks = [command for operation, command in events
                  if operation == "output" and command[0] == "psql"]
        self.assertEqual(len(checks), 1)
        self.assertEqual(checks[0][-2:], ["-c", "SHOW max_locks_per_transaction"])
        self.assertIn("-At", checks[0])
        self.assertIn("-X", checks[0])
        self.assertIn("ON_ERROR_STOP=1", checks[0])
        self.assertTrue(environment["CASE_TEST_DATABASE_URL"].endswith("/case_tests"))

    def test_effective_lock_capacity_mismatch_rejects_before_work(self):
        with startup_backend(64) as (backend, events), patch.object(
            MODULE, "Backend", return_value=backend
        ), patch.object(backend, "stop") as stop, patch.object(
            MODULE.subprocess, "Popen"
        ) as spawn:
            with self.assertRaisesRegex(RuntimeError, "lock"):
                MODULE.run(["test-command"])
            spawn.assert_not_called()
            stop.assert_called_once()
        self.assertTrue(any(operation == "output" and command[0] == "psql"
                            for operation, command in events))
        self.assertFalse(any(operation == "run" and command[0] in {"bash", "psql"}
                             for operation, command in events))
    def test_failed_setup_still_stops_its_service(self):
        backend = Mock()
        backend.start.side_effect = RuntimeError("startup failed")
        with patch.object(MODULE, "Backend", return_value=backend), self.assertRaises(RuntimeError):
            MODULE.run(["test-command"])
        backend.stop.assert_called_once()

    def test_failed_test_preserves_exit_code_and_stops_service(self):
        backend = Mock()
        backend.start.return_value = {"CASE_TEST_DATABASE_URL": "isolated"}
        child = Mock()
        child.wait.return_value = 7
        child.poll.return_value = 7
        with patch.object(MODULE, "Backend", return_value=backend), patch.object(
            MODULE.subprocess, "Popen", return_value=child
        ) as spawn:
            self.assertEqual(MODULE.run(["test-command"]), 7)
        self.assertEqual(spawn.call_args.kwargs["env"]["CASE_TEST_DATABASE_URL"], "isolated")
        self.assertTrue(spawn.call_args.kwargs["start_new_session"])
        backend.stop.assert_called_once()

    def test_cancellation_terminates_children_before_stopping_database(self):
        events = []
        backend = Mock()
        backend.stop.side_effect = lambda: events.append("database")
        child = Mock(pid=1234)
        child.wait.side_effect = [SystemExit(143), 0]
        child.poll.return_value = None
        with patch.object(MODULE, "Backend", return_value=backend), patch.object(
            MODULE.subprocess, "Popen", return_value=child
        ), patch.object(MODULE.os, "killpg", side_effect=lambda *args: events.append("children")):
            with self.assertRaises(SystemExit) as raised:
                MODULE.run(["test-command"])
        self.assertEqual(raised.exception.code, 143)
        self.assertEqual(events, ["children", "database"])

    def test_cleanup_refuses_a_marker_outside_its_owned_namespace(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            marker = root / "state.json"
            marker.write_text('{"unit":"unrelated.service","directory":"/tmp"}')
            with patch.dict(MODULE.os.environ, {"RUNNER_TEMP": str(root)}), self.assertRaises(ValueError):
                MODULE.cleanup(marker)
            self.assertTrue(marker.exists())

    def test_sigterm_produces_cancellation_exit_status(self):
        with self.assertRaises(SystemExit) as raised:
            MODULE.cancel(signal.SIGTERM, None)
        self.assertEqual(raised.exception.code, 143)


if __name__ == "__main__":
    unittest.main()
