"""A generation descriptor ends with its owned Python execution boundary."""
import json
import signal
import sys
import unittest

from deploy_controller_launcher_programs import (
    EXEC_ENTRY, LIFECYCLE_DRIVER, LIFECYCLE_ENTRY, SIGNAL_ENTRY,
)
from deploy_controller_launcher_support import Fixture, LAUNCHER, LauncherTests, close_child


class ControllerLauncherLifecycleTests(LauncherTests):
    def test_return_exception_and_system_exit_close_descriptors_and_restore_caller_arguments(self):
        fixture = Fixture(self, LIFECYCLE_ENTRY)
        for outcome, kind, status in (("return", "returned", None),
                                      ("error", "RuntimeError", None), ("exit", "SystemExit", 23)):
            with self.subTest(outcome=outcome):
                command = [sys.executable, "-I", "-B", "-S", "-u", "-c", LIFECYCLE_DRIVER,
                           str(LAUNCHER), str(fixture.root), fixture.expected, outcome]
                code, output, error, _pid = self.run_child(fixture, command)
                self.assertEqual((code, error), (0, ""))
                result = json.loads(output)
                self.assertEqual((result["kind"], result["status"]), (kind, status))
                self.assertEqual(result["after"], result["before"])
                self.assertTrue(result["argv_restored"])
                self.assertTrue(result["path_restored"])
                fixture.preserved(self)

    def test_exec_replaces_the_same_process_without_inheriting_the_generation_descriptor(self):
        fixture = Fixture(self, EXEC_ENTRY)
        arguments = ["literal value", "$(not-a-command)", "--", ""]
        code, output, error, pid = self.run_child(fixture, fixture.command(arguments=arguments))
        self.assertEqual((code, error), (0, ""))
        result = json.loads(output)
        self.assertEqual(result, {"inherited": False, "pid": pid, "arguments": arguments,
                                  "environment": fixture.environment["CONTROLLER_FIXTURE_VALUE"]})
        fixture.preserved(self)

    def test_signal_targets_the_controller_process_without_a_launcher_proxy_or_orphan(self):
        fixture = Fixture(self, SIGNAL_ENTRY)
        process = fixture.start(fixture.command())
        try:
            self.assertEqual(fixture.ready(self, process), {"ready": True, "pid": process.pid})
            process.send_signal(signal.SIGTERM)
            output, error = fixture.completed(self, process)
            self.assertEqual((process.returncode, output, error), (-signal.SIGTERM, "", ""))
            fixture.preserved(self)
        finally:
            close_child(process)


if __name__ == "__main__":
    unittest.main()
