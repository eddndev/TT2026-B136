"""Check cancellation and failure cleanup for disposable CI services."""

from contextlib import contextmanager
import fcntl
import importlib.util
import json
import multiprocessing
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


@contextmanager
def cleanup_backend():
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        directory = root / "tt-ci-postgres-owned"
        (directory / "data").mkdir(parents=True)
        (directory / "data" / "postgresql.conf").write_text("shared_buffers=128MB\n")
        marker = root / "tt-ci-postgres.json"
        marker.write_text(json.dumps({
            "unit": "tt-ci-postgres-" + "a" * 32,
            "directory": str(directory),
        }))
        marker.chmod(0o600)
        with patch.dict(MODULE.os.environ, {"RUNNER_TEMP": str(root)}):
            yield marker, directory


def concurrent_cleanup_worker(marker, results):
    try:
        MODULE.cleanup(marker)
    except BaseException as error:
        results.put((multiprocessing.current_process().name, type(error).__name__, str(error)))
    else:
        results.put((multiprocessing.current_process().name, None, None))


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

    def test_concurrent_cleanup_serializes_marker_read_stop_and_directory_removal(self):
        context = multiprocessing.get_context("fork")
        first_at_remove = context.Event()
        release_first = context.Event()
        results = context.Queue()
        removals = context.Queue()
        stops = context.Queue()
        real_remove = MODULE.shutil.rmtree
        real_flock = fcntl.flock

        def remove(path, *args, **kwargs):
            name = multiprocessing.current_process().name
            removals.put(name)
            if name == "cleanup-first":
                first_at_remove.set()
                if not release_first.wait(5):
                    raise TimeoutError("second cleanup did not reach the shared boundary")
                return real_remove(path, *args, **kwargs)
            try:
                return real_remove(path, *args, **kwargs)
            finally:
                release_first.set()

        def lock(descriptor, operation):
            if (multiprocessing.current_process().name == "cleanup-second"
                    and operation == fcntl.LOCK_EX):
                release_first.set()
            return real_flock(descriptor, operation)

        def stop(command, **_kwargs):
            self.assertEqual(command[:3], ["systemctl", "--user", "stop"])
            stops.put(multiprocessing.current_process().name)
            return Mock(returncode=0)

        with cleanup_backend() as (marker, directory):
            children = [context.Process(target=concurrent_cleanup_worker,
                        args=(marker, results), name=f"cleanup-{name}")
                        for name in ["first", "second"]]
            with patch.object(MODULE.subprocess, "run", side_effect=stop), patch.object(
                MODULE.shutil, "rmtree", side_effect=remove
            ), patch.object(fcntl, "flock", side_effect=lock):
                try:
                    children[0].start()
                    self.assertTrue(first_at_remove.wait(5), "first cleanup did not reach removal")
                    children[1].start()
                    outcomes = [results.get(timeout=5), results.get(timeout=5)]
                finally:
                    release_first.set()
                    for child in children:
                        if child.pid is not None:
                            child.join(5)
                            if child.is_alive():
                                child.terminate()
                                child.join(5)
            self.assertTrue(all(child.exitcode == 0 for child in children))
            self.assertEqual(sorted(outcomes), [("cleanup-first", None, None), ("cleanup-second", None, None)])
            self.assertEqual(stops.get(timeout=1), "cleanup-first")
            self.assertTrue(stops.empty(), "the same unit must be stopped only once")
            self.assertEqual(removals.get(timeout=1), "cleanup-first")
            self.assertTrue(removals.empty(), "the same tree must be removed only once")
            self.assertFalse(marker.exists())
            self.assertFalse(directory.exists())
        for queue in [results, removals, stops]:
            queue.close()
            queue.join_thread()

    def test_cleanup_failed_service_stop_retains_marker_and_data(self):
        with cleanup_backend() as (marker, directory), patch.object(
            MODULE.subprocess, "run", return_value=Mock(returncode=1)
        ), patch.object(MODULE.subprocess, "check_output", return_value="loaded\n"):
            with self.assertRaisesRegex(RuntimeError, "retaining its data"):
                MODULE.cleanup(marker)
            self.assertTrue(marker.is_file())
            self.assertEqual((directory / "data" / "postgresql.conf").read_text(), "shared_buffers=128MB\n")

    def test_cleanup_permission_failure_is_not_silenced_or_unmarked(self):
        with cleanup_backend() as (marker, directory), patch.object(
            MODULE.subprocess, "run", return_value=Mock(returncode=0)
        ), patch.object(MODULE.shutil, "rmtree", side_effect=PermissionError("denied")):
            with self.assertRaisesRegex(PermissionError, "denied"):
                MODULE.cleanup(marker)
            self.assertTrue(marker.is_file())
            self.assertTrue((directory / "data" / "postgresql.conf").is_file())

    def test_cleanup_valid_unit_cannot_remove_an_unrelated_directory(self):
        with cleanup_backend() as (marker, directory), tempfile.TemporaryDirectory() as other:
            state = json.loads(marker.read_text())
            state["directory"] = other
            marker.write_text(json.dumps(state))
            with patch.object(MODULE.subprocess, "run") as stop:
                with self.assertRaisesRegex(ValueError, "unrelated"):
                    MODULE.cleanup(marker)
                stop.assert_not_called()
            self.assertTrue(marker.is_file())
            self.assertTrue(directory.is_dir())
            self.assertTrue(Path(other).is_dir())

    def test_sigterm_produces_cancellation_exit_status(self):
        with self.assertRaises(SystemExit) as raised:
            MODULE.cancel(signal.SIGTERM, None)
        self.assertEqual(raised.exception.code, 143)


if __name__ == "__main__":
    unittest.main()
