"""A pending CRL transition blocks boot, readiness and release changes."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
import release
import runtime


class CrlFenceTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        (self.root / "config").mkdir()
        (self.root / "data").mkdir()
        (self.root / "releases").mkdir()
        self.current = self.make_release("current", "v1.1.0")
        self.previous = self.make_release("previous", "v1.0.0")
        self.next = self.make_release("next", "v1.2.0")
        (self.root / "current").symlink_to(self.current)
        (self.root / "previous").symlink_to(self.previous)
        (self.root / "config/schema").write_text("same")
        (self.root / "config/settings.json").write_text(json.dumps({"api_port": 19081}))
        self.fence = self.root / "config/crl-maintenance.json"
        self.fence.write_text(json.dumps({"operation_id": "10000000-0000-4000-8000-000000000001"}))

    def make_release(self, name, version):
        target = self.root / "releases" / name
        target.mkdir()
        (target / "release.json").write_text(json.dumps(
            {"version": version, "commit": "a" * 40, "schema": "same"}))
        return target

    def test_serve_rejects_a_pending_operation_before_exec_or_loading_secrets(self):
        with patch.object(runtime, "environment", return_value={}) as env:
            with patch.object(runtime.os, "chdir"), patch.object(runtime.os, "execve") as execute:
                with self.assertRaisesRegex(RuntimeError, "maintenance"):
                    runtime.serve(self.root)
                execute.assert_not_called()
                env.assert_not_called()

    def test_malformed_fence_does_not_make_boot_safe(self):
        for content in ("", "invalid JSON", "{}"):
            with self.subTest(content=content):
                self.fence.write_text(content)
                with patch.object(runtime, "environment", return_value={}):
                    with patch.object(runtime.os, "chdir"), patch.object(runtime.os, "execve") as execute:
                        with self.assertRaisesRegex(RuntimeError, "maintenance"):
                            runtime.serve(self.root)
                        execute.assert_not_called()

    def test_wait_api_cannot_authorize_web_start_during_maintenance(self):
        target = runtime.Runtime(self.root)
        with patch.object(target, "dependencies") as dependencies:
            with patch.object(target, "api_ready", return_value=True) as ready:
                with self.assertRaisesRegex(RuntimeError, "maintenance"):
                    target.wait_api()
                dependencies.assert_not_called()
                ready.assert_not_called()

    def test_activation_rejects_fence_before_stop_backup_initialize_or_link_change(self):
        target = Mock()
        with self.assertRaisesRegex(RuntimeError, "maintenance"):
            release.activate(self.root, self.next, target)
        self.assertEqual(target.mock_calls, [])
        self.assertEqual((self.root / "current").resolve(), self.current)
        self.assertEqual((self.root / "previous").resolve(), self.previous)

    def test_rollback_rejects_fence_before_stop_backup_initialize_or_link_change(self):
        target = Mock()
        with self.assertRaisesRegex(RuntimeError, "maintenance"):
            release.rollback(self.root, target)
        self.assertEqual(target.mock_calls, [])
        self.assertEqual((self.root / "current").resolve(), self.current)
        self.assertEqual((self.root / "previous").resolve(), self.previous)

    def test_absent_fence_keeps_the_existing_readiness_contract(self):
        self.fence.unlink()
        target = runtime.Runtime(self.root)
        with patch.object(target, "dependencies") as dependencies:
            with patch.object(target, "api_ready", return_value=True) as ready:
                target.wait_api()
                dependencies.assert_called_once_with()
                ready.assert_called_once_with()


if __name__ == "__main__":
    unittest.main()
