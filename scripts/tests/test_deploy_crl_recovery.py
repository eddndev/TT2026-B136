"""Recovery requires the recorded backup and reconciles partial durable writes."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

from test_deploy_crl_support import CrlFixture, renew_crl


class CrlBackupRecoveryTests(CrlFixture):
    def committed_operation(self):
        self.publish_error = "after"
        with self.assertRaisesRegex(RuntimeError, "publication"):
            self.renew()
        self.assertEqual(self.publications, [3])
        self.assertFalse(self.runtime.running)
        return self.root / "backups/original"

    def assert_invalid_backup_blocks_resume(self):
        head = copy.deepcopy(self.head)
        counter = self.counter.read_bytes()
        with self.assertRaises((OSError, ValueError, RuntimeError)):
            self.resume()
        self.assertFalse(self.runtime.running)
        self.assertTrue(self.fence.is_file())
        self.assertEqual(self.head, head)
        self.assertEqual(self.counter.read_bytes(), counter)
        self.assertEqual(self.active_crl.read_bytes(), b"candidate CRL")
        self.assertEqual(self.publications, [3])
        self.assertEqual(self.runtime.events.count("backup"), 1)
        self.assertNotIn("start", self.runtime.events)

    def test_committed_resume_rejects_a_missing_backup_completion_marker(self):
        backup = self.committed_operation()
        (backup / "COMPLETE").unlink()
        self.assert_invalid_backup_blocks_resume()

    def test_committed_resume_rejects_a_missing_recorded_redis_snapshot(self):
        backup = self.committed_operation()
        (backup / "redis.rdb").unlink()
        self.assert_invalid_backup_blocks_resume()

    def test_committed_resume_rejects_a_changed_dump_even_when_size_is_unchanged(self):
        backup = self.committed_operation()
        path = backup / "database.dump"
        before = path.read_bytes()
        path.write_bytes(bytes([before[0] ^ 1]) + before[1:])
        self.assertEqual(path.stat().st_size, len(before))
        self.assert_invalid_backup_blocks_resume()

    def test_committed_resume_rejects_a_private_archive_replaced_by_a_symlink(self):
        backup = self.committed_operation()
        path = backup / "private-state.tar.gz"
        other = self.root / "unrelated-private-state.tar.gz"
        other.write_bytes(path.read_bytes())
        path.unlink()
        path.symlink_to(other)
        self.assert_invalid_backup_blocks_resume()


class CrlDurabilityRecoveryTests(CrlFixture):
    def interrupted_write(self, boundary, committed, installed):
        journal = renew_crl.journal
        original_replace = journal.os.replace
        original_sync = journal.sync_directory
        injected = False
        accepted_directory = None

        def replace(source, destination):
            nonlocal injected, accepted_directory
            destination = Path(destination)
            fail = (boundary == "crl-replace" and destination == self.active_crl)
            if destination.name == "journal.json":
                state = json.loads(Path(source).read_text())["state"]
                fail = fail or (boundary == "committed-replace" and state == "committed")
                if boundary == "accepted-sync" and state == "accepted":
                    accepted_directory = destination.parent
            if fail and not injected:
                injected = True
                raise OSError("injected persistence failure")
            return original_replace(source, destination)

        def sync(directory):
            nonlocal injected
            directory = Path(directory)
            fail = ((boundary == "counter-sync" and directory == self.counter.parent
                     and int(self.counter.read_text(), 16) == 4098)
                    or (boundary == "crl-sync" and directory == self.active_crl.parent
                        and self.active_crl.read_bytes() == b"candidate CRL")
                    or (boundary == "accepted-sync" and directory == accepted_directory))
            if fail and not injected:
                injected = True
                raise OSError("injected persistence failure")
            return original_sync(directory)

        with patch.object(journal.os, "replace", side_effect=replace):
            with patch.object(journal, "sync_directory", side_effect=sync):
                with self.assertRaisesRegex(OSError, "injected persistence failure"):
                    self.renew()
        self.assertTrue(injected)
        self.assertTrue(self.fence.is_file())
        self.assertFalse(self.runtime.running)
        self.assertEqual(self.head["revision"], 4 if committed else 3)
        self.assertEqual(self.publications, [3] if committed else [])
        self.assertEqual(int(self.counter.read_text(), 16), 4098)
        self.assertEqual(self.active_crl.read_bytes(), b"candidate CRL" if installed else b"original CRL")
        self.assertEqual(self.resume()["revision"], 4)
        self.assertEqual(self.publications, [3])
        self.assertEqual(len(self.generated), 1)
        self.assertEqual(self.runtime.events.count("backup"), 1)
        self.assertTrue(self.runtime.running)
        self.assertFalse(self.fence.exists())
        self.assertEqual(int(self.counter.read_text(), 16), 4098)
        self.assert_keys_preserved()

    def test_counter_directory_flush_failure_keeps_counter_advanced_and_can_resume(self):
        self.interrupted_write("counter-sync", committed=False, installed=False)

    def test_crl_replace_failure_after_counter_install_can_resume_without_another_candidate(self):
        self.interrupted_write("crl-replace", committed=False, installed=False)

    def test_crl_directory_flush_failure_keeps_fence_until_same_candidate_is_published(self):
        self.interrupted_write("crl-sync", committed=False, installed=True)

    def test_committed_journal_replace_failure_resumes_without_republishing(self):
        self.interrupted_write("committed-replace", committed=True, installed=True)

    def test_acceptance_flush_failure_stops_and_rechecks_committed_state_on_resume(self):
        self.interrupted_write("accepted-sync", committed=True, installed=True)


if __name__ == "__main__":
    unittest.main()
