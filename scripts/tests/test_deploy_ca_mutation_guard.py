import json
from unittest.mock import Mock, patch

from ca_issuance_support import IssuanceFixture, OPERATION
import crl_journal
import release
import renew_crl
import restore_fence
import runtime


class CaMutationGuardTests(IssuanceFixture):
    def test_unfinished_portal_mutations_block_other_ca_or_crl_operations(self):
        for group in ("operations", "revocations", "publications"):
            path = self.ca / group / OPERATION
            path.mkdir(parents=True)
            journal = path / "journal.json"
            journal.write_text(json.dumps({"receipt": None}))
            with self.subTest(group=group):
                with self.assertRaises(RuntimeError):
                    crl_journal.guard_mutation(self.root)
                crl_journal.guard_mutation(self.root, ca_operation=(group, OPERATION))
                with self.assertRaises(RuntimeError):
                    crl_journal.guard_mutation(self.root, ca_operation=("wrong-kind", OPERATION))
                journal.write_text(json.dumps({"receipt": {"state": "done"}}))
                crl_journal.guard_mutation(self.root)

    def test_missing_or_linked_journal_is_not_assumed_completed(self):
        path = self.ca / "operations" / OPERATION
        path.mkdir(parents=True)
        with self.assertRaises((ValueError, OSError, RuntimeError)):
            crl_journal.guard_mutation(self.root)
        (path / "journal.json").symlink_to(self.ca / "serial")
        with self.assertRaises((ValueError, OSError, RuntimeError)):
            crl_journal.guard_mutation(self.root)

    def pending_publication(self):
        path = self.ca / "publications" / OPERATION
        path.mkdir(parents=True)
        (path / "journal.json").write_text(json.dumps({"receipt": None}))

    def test_pending_portal_operation_does_not_close_normal_startup_without_fence(self):
        self.pending_publication()
        (self.root / "config/settings.json").write_text(json.dumps({"api_port": 19081}))
        target = self.root / "releases/test"
        target.mkdir(parents=True)
        (self.root / "current").symlink_to(target)
        service = runtime.Runtime(self.root)
        with patch.object(service, "dependencies"), patch.object(service, "api_ready", return_value=True):
            service.wait_api()
        with patch.object(service, "wait_api") as wait, patch.object(service, "check"):
            with patch.object(runtime, "run") as execute:
                service.start(target)
            wait.assert_called_once_with()
            self.assertEqual(execute.call_count, 3)
        with patch.object(runtime, "environment", return_value={}):
            with patch.object(runtime.os, "chdir"), patch.object(runtime.os, "execve") as execute:
                runtime.serve(self.root)
                execute.assert_called_once()

    def test_pending_portal_operation_still_blocks_release_crl_and_restore_mutations(self):
        self.pending_publication()
        service = Mock()
        target = self.root / "releases/test"
        actions = (
            lambda: release.activate(self.root, target, service),
            lambda: release.rollback(self.root, service),
            lambda: release.admit(self.root, target, "a" * 64, "v1.0.0", "a" * 40),
            lambda: renew_crl.renew(self.root, 1, runtime=service),
            lambda: renew_crl.resume(self.root, OPERATION, runtime=service),
            lambda: restore_fence.enter(self.root, OPERATION),
        )
        for action in actions:
            with self.subTest(action=action), self.assertRaisesRegex(RuntimeError, "pending"):
                action()
        self.assertEqual(service.mock_calls, [])
        self.assertFalse(restore_fence.fence_path(self.root).exists())

    def test_generic_backup_rejects_portal_pending_before_any_capture_command(self):
        self.pending_publication()
        (self.root / "config/settings.json").write_text("{}")
        service = runtime.Runtime(self.root)
        with patch.object(runtime, "run") as execute:
            with self.assertRaisesRegex(RuntimeError, "pending"):
                service.backup()
            execute.assert_not_called()
