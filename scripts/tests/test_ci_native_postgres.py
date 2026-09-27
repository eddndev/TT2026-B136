"""Check cancellation and failure cleanup for disposable CI services."""

import importlib.util
from pathlib import Path
import signal
import tempfile
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "ci_native_postgres", ROOT / "scripts/ci_native_postgres.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class TestNativePostgres(unittest.TestCase):
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
