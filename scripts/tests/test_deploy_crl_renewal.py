"""CRL renewal preserves identity and stops on unconfirmed durable state."""
import copy
import fcntl
import json
import os
import unittest

from test_deploy_crl_support import CrlFixture, renew_crl


class CrlRenewalTests(CrlFixture):
    def test_renewal_advances_trust_once_after_backup_without_replacing_keys(self):
        result = self.renew()
        self.assertEqual(result["revision"], 4)
        self.assertEqual(self.head["deployment_id"], self.baseline["deployment_id"])
        self.assertEqual(self.publications, [3])
        self.assertEqual(self.runtime.events[:2], ["stop", "backup"])
        self.assertTrue(self.runtime.running)
        self.assertFalse(self.fence.exists())
        self.assertEqual(int(self.counter.read_text(), 16), 4098)
        self.assert_keys_preserved()

    def test_wrong_expected_revision_does_not_stop_generate_or_publish(self):
        with self.assertRaisesRegex((ValueError, RuntimeError), "revision"):
            self.renew(expected=2)
        self.assertEqual(self.runtime.events, [])
        self.assertEqual(self.generated, [])
        self.assertEqual(self.publications, [])
        self.assertEqual(self.active_crl.read_bytes(), b"original CRL")
        self.assertEqual(self.counter.read_bytes(), b"1001\n")

    def test_release_schema_must_match_the_initialized_database(self):
        (self.root / "config/schema").write_text("c" * 64 + "\n")
        with self.assertRaisesRegex((ValueError, RuntimeError), "schema"):
            self.renew()
        self.assertEqual(self.runtime.events, [])
        self.assertEqual(self.generated, [])
        self.assertEqual(self.publications, [])

    def test_deployment_lock_excludes_renewal_without_touching_services(self):
        with (self.root / "deploy.lock").open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            with self.assertRaises((BlockingIOError, RuntimeError)):
                self.renew()
        self.assertEqual(self.runtime.events, [])
        self.assertEqual(self.generated, [])
        self.assertEqual(self.publications, [])

    def test_unknown_sql_state_never_generates_installs_or_starts(self):
        self.query_error = True
        with self.assertRaisesRegex(RuntimeError, "database"):
            self.renew()
        self.assertEqual(self.runtime.events, [])
        self.assertEqual(self.generated, [])
        self.assertEqual(self.active_crl.read_bytes(), b"original CRL")

    def test_missing_local_crl_is_not_silently_regenerated(self):
        self.active_crl.unlink()
        with self.assertRaises((OSError, ValueError, RuntimeError)):
            self.renew()
        self.assertEqual(self.generated, [])
        self.assertEqual(self.publications, [])
        self.assertEqual(self.runtime.events, [])

    def test_failed_backup_cannot_install_or_publish_the_candidate(self):
        self.runtime.fail_backup = True
        with self.assertRaisesRegex(RuntimeError, "backup"):
            self.renew()
        self.assertEqual(self.active_crl.read_bytes(), b"original CRL")
        self.assertEqual(self.publications, [])
        self.assert_keys_preserved()

    def test_failure_before_commit_stays_offline_and_resume_publishes_the_same_candidate(self):
        self.publish_error = "before"
        with self.assertRaisesRegex(RuntimeError, "publication"):
            self.renew()
        self.assertFalse(self.runtime.running)
        self.assertTrue(self.fence.is_file())
        self.assertEqual(self.head, self.baseline)
        self.assertEqual(self.active_crl.read_bytes(), b"candidate CRL")
        counter = self.counter.read_bytes()
        self.publish_error = None
        self.assertEqual(self.resume()["revision"], 4)
        self.assertEqual(self.publications, [3, 3])
        self.assertEqual(len(self.generated), 1)
        self.assertEqual(self.counter.read_bytes(), counter)
        self.assertTrue(self.runtime.running)
        self.assert_keys_preserved()

    def test_lost_committed_response_is_reconciled_without_a_second_publication(self):
        self.publish_error = "after"
        with self.assertRaisesRegex(RuntimeError, "publication"):
            self.renew()
        self.assertEqual(self.head["revision"], 4)
        self.assertFalse(self.runtime.running)
        self.assertTrue(self.fence.is_file())
        self.assertEqual(self.resume()["revision"], 4)
        self.assertEqual(self.publications, [3])
        self.assertEqual(len(self.generated), 1)
        self.assertTrue(self.runtime.running)
        self.assert_keys_preserved()

    def test_health_failure_never_rolls_back_committed_trust_or_the_counter(self):
        self.runtime.fail_start = True
        with self.assertRaisesRegex(RuntimeError, "health"):
            self.renew()
        committed = copy.deepcopy(self.head)
        counter = self.counter.read_bytes()
        self.assertFalse(self.runtime.running)
        self.assertTrue(self.fence.is_file())
        self.assertEqual(self.active_crl.read_bytes(), b"candidate CRL")
        self.assertEqual(committed["revision"], 4)
        self.runtime.fail_start = False
        self.assertEqual(self.resume()["revision"], 4)
        self.assertEqual(self.head, committed)
        self.assertEqual(self.counter.read_bytes(), counter)
        self.assertEqual(self.publications, [3])
        self.assert_keys_preserved()

    def test_unrelated_committed_head_blocks_resume_instead_of_adopting_it(self):
        self.publish_error = "after"
        with self.assertRaises(RuntimeError):
            self.renew()
        for field, value in (("deployment_id", "20000000-0000-4000-8000-000000000002"),
                             ("revision", 5), ("crl_der", b"other DER".hex()),
                             ("root_der", b"other root".hex()), ("crl_number", 4098)):
            with self.subTest(field=field):
                original = self.head[field]
                self.head[field] = value
                with self.assertRaises((ValueError, RuntimeError)):
                    self.resume()
                self.head[field] = original
                self.assertFalse(self.runtime.running)
                self.assertTrue(self.fence.exists())
                self.assertEqual(self.publications, [3])

    def test_corrupted_candidate_cannot_be_accepted_only_because_the_journal_claims_commit(self):
        self.publish_error = "after"
        with self.assertRaises(RuntimeError):
            self.renew()
        (self.generated[0] / "candidate.pem").write_bytes(b"corrupted CRL")
        with self.assertRaises((ValueError, RuntimeError)):
            self.resume()
        self.assertFalse(self.runtime.running)
        self.assertTrue(self.fence.exists())
        self.assertEqual(self.publications, [3])

    def test_resume_rechecks_installed_crl_instead_of_trusting_only_saved_candidate(self):
        self.publish_error = "after"
        with self.assertRaises(RuntimeError):
            self.renew()
        self.active_crl.write_bytes(b"foreign installed CRL")
        with self.assertRaises((ValueError, RuntimeError)):
            self.resume()
        self.assertFalse(self.runtime.running)
        self.assertTrue(self.fence.exists())
        self.assertEqual(self.publications, [3])

    def test_private_journal_candidate_and_originals_ignore_permissive_umask(self):
        self.publish_error = "after"
        before = os.umask(0)
        try:
            with self.assertRaises(RuntimeError):
                self.renew()
        finally:
            os.umask(before)
        operation = self.generated[0]
        self.assertEqual(operation.stat().st_mode & 0o777, 0o700)
        self.assertEqual(self.fence.stat().st_mode & 0o777, 0o600)
        journal = json.loads((operation / "journal.json").read_text())
        self.assertIsInstance(journal, dict)
        for path in operation.rglob("*"):
            self.assertFalse(path.is_symlink())
            self.assertEqual(path.stat().st_mode & 0o777, 0o700 if path.is_dir() else 0o600)

    def test_existing_pending_operation_is_not_overwritten_by_another_renew(self):
        self.publish_error = "after"
        with self.assertRaises(RuntimeError):
            self.renew()
        fence = self.fence.read_bytes()
        with self.assertRaises((ValueError, RuntimeError)):
            self.renew(expected=4)
        self.assertEqual(self.fence.read_bytes(), fence)
        self.assertEqual(self.publications, [3])
        self.assertEqual(len(self.generated), 1)

    def test_resume_rejects_paths_outside_the_operation_directory(self):
        for operation in ("../other", "/tmp/other", "", "a/b"):
            with self.subTest(operation=operation):
                with self.assertRaises((ValueError, RuntimeError)):
                    renew_crl.resume(self.root, operation, runtime=self.runtime)
                self.assertEqual(self.runtime.events, [])


if __name__ == "__main__":
    unittest.main()
