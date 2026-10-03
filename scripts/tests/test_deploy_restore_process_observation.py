"""Keep process-exit assertions reliable when procfs entries disappear."""
from unittest import TestCase
from unittest.mock import Mock

from test_deploy_restore_commands import process_is_running


class RestoreProcessObservationTests(TestCase):
    def test_disappearance_during_read_means_process_has_exited(self):
        for failure in (FileNotFoundError(), ProcessLookupError()):
            with self.subTest(failure=type(failure).__name__):
                status = Mock()
                status.read_text.side_effect = failure
                self.assertFalse(process_is_running(status))
                status.read_text.assert_called_once_with()
                status.exists.assert_not_called()

    def test_live_and_zombie_states_are_distinguished(self):
        for state, expected in (("S", True), ("R", True), ("Z", False)):
            with self.subTest(state=state):
                status = Mock()
                status.read_text.return_value = f"123 (python worker) {state} 1 2 3"
                self.assertEqual(process_is_running(status), expected)

    def test_unexpected_read_failure_is_not_treated_as_exit(self):
        status = Mock()
        status.read_text.side_effect = PermissionError()
        with self.assertRaises(PermissionError):
            process_is_running(status)
