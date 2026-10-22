"""Scheduled CRL renewal acts only near expiry and never resumes on its own."""
import fcntl
import json
from pathlib import Path
import unittest
from unittest.mock import patch

from test_deploy_crl_support import CrlFixture, renew_crl
import crl_schedule


HOUR = 3600
PIN = "c" * 64


class CrlScheduleTests(CrlFixture):
    def auto(self, threshold=72 * HOUR, keep=4):
        with patch.object(crl_schedule.time, "time", return_value=self.now):
            return crl_schedule.run(self.root, threshold, keep, runtime=self.runtime)

    def receipts(self):
        directory = self.root / "maintenance/crl-schedule"
        return [json.loads(path.read_text()) for path in sorted(directory.glob("*.json"))]

    def test_valid_crl_beyond_the_threshold_is_left_untouched(self):
        self.head.update(valid_until=self.now + 5 * 86400)
        receipt = self.auto()
        self.assertEqual(receipt["result"], "skipped")
        self.assertEqual(crl_schedule.exit_code(receipt), 0)
        self.assertEqual(self.runtime.events, [])
        self.assertEqual(self.generated, [])
        self.assertEqual(self.publications, [])
        self.assertEqual(self.receipts(), [receipt])

    def test_crl_near_expiry_is_renewed_from_the_observed_revision(self):
        receipt = self.auto()
        self.assertEqual(receipt["result"], "renewed")
        self.assertEqual(crl_schedule.exit_code(receipt), 0)
        self.assertEqual(self.publications, [3])
        self.assertEqual((receipt["revision_before"], receipt["revision_after"]), (3, 4))
        self.assertEqual(receipt["crl_number_after"], 4097)
        self.assertEqual(receipt["backup"], str(self.root / "backups/original"))
        self.assertEqual(receipt["operation_id"], self.generated[0].name)
        self.assertTrue(self.runtime.running)
        self.assertFalse(self.fence.exists())
        self.assert_keys_preserved()

    def test_pending_maintenance_blocks_without_stopping_or_resuming(self):
        operation = "10000000-0000-4000-8000-000000000001"
        self.fence.write_text(json.dumps({"operation_id": operation}))
        receipt = self.auto()
        self.assertEqual(receipt["result"], "blocked")
        self.assertNotEqual(crl_schedule.exit_code(receipt), 0)
        self.assertEqual(self.runtime.events, [])
        self.assertEqual(self.publications, [])
        self.assertEqual(json.loads(self.fence.read_text()), {"operation_id": operation})

    def test_held_deployment_lock_is_a_retryable_outcome_without_side_effects(self):
        self.head.update(valid_until=self.now + 5 * 86400)
        with (self.root / "deploy.lock").open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            receipt = self.auto()
        self.assertEqual(receipt["result"], "busy")
        self.assertEqual(crl_schedule.exit_code(receipt), crl_schedule.EXIT_BUSY)
        self.assertEqual(self.runtime.events, [])
        self.assertEqual(self.generated, [])

    def test_held_lock_near_expiry_fails_visibly_without_side_effects(self):
        with (self.root / "deploy.lock").open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            receipt = self.auto()
        self.assertEqual(receipt["result"], "busy_near_expiry")
        self.assertEqual(crl_schedule.exit_code(receipt), 1)
        self.assertEqual(self.runtime.events, [])

    def test_interruption_during_renewal_is_recorded_and_stays_fenced(self):
        def interrupted(*, ca_operation=None):
            self.runtime.events.append("backup")
            raise KeyboardInterrupt
        self.runtime.backup = interrupted
        receipt = self.auto()
        self.assertEqual(receipt["result"], "interrupted")
        self.assertEqual(crl_schedule.exit_code(receipt), 1)
        self.assertTrue(self.fence.exists())
        self.assertFalse(self.runtime.running)
        self.assertEqual(self.publications, [])
        self.assertEqual(self.receipts()[-1]["result"], "interrupted")

    def test_authority_expiry_inside_the_threshold_alerts_instead_of_renewing(self):
        self.head.update(crl_next_update=self.now + 7 * 86400, valid_until=self.now + 86400)
        receipt = self.auto()
        self.assertEqual(receipt["result"], "ca_expiring")
        self.assertEqual(crl_schedule.exit_code(receipt), 1)
        self.assertEqual(self.runtime.events, [])
        self.assertEqual(self.generated, [])

    def test_retention_failure_does_not_hide_an_accepted_renewal(self):
        with patch.object(crl_schedule, "prune", side_effect=OSError("disk")):
            receipt = self.auto()
        self.assertEqual(receipt["result"], "renewed")
        self.assertEqual(receipt["post_error"], "OSError")
        self.assertEqual(crl_schedule.exit_code(receipt), 0)
        self.assertEqual(self.publications, [3])

    def test_scheduled_retention_counts_the_current_backup(self):
        earlier = []
        for index in range(5):
            operation = f"30000000-0000-4000-8000-{index:012d}"
            backup = self.root / "backups" / f"earlier{index}"
            backup.mkdir(mode=0o700, parents=True)
            journal = self.root / "maintenance/crl" / operation
            journal.mkdir(mode=0o700, parents=True)
            (journal / "journal.json").write_text(json.dumps(
                {"operation_id": operation, "state": "accepted", "backup": {"path": str(backup)}}))
            receipts = self.root / "maintenance/crl-schedule"
            receipts.mkdir(mode=0o700, parents=True, exist_ok=True)
            (receipts / f"{index + 1}-{index:08x}.json").write_text(json.dumps(
                {"checked_at": index + 1, "result": "renewed", "operation_id": operation,
                 "backup": str(backup)}))
            earlier.append(backup)
        receipt = self.auto(keep=4)
        self.assertEqual(receipt["result"], "renewed")
        self.assertEqual([path.exists() for path in earlier], [False, False, True, True, True])
        self.assertTrue(Path(receipt["backup"]).exists())

    def test_failed_renewal_stays_fenced_and_the_next_trigger_does_not_resume(self):
        self.publish_error = "before"
        receipt = self.auto()
        self.assertEqual(receipt["result"], "failed")
        self.assertEqual(receipt["error"], "RuntimeError")
        self.assertEqual(crl_schedule.exit_code(receipt), 1)
        self.assertFalse(self.runtime.running)
        self.assertTrue(self.fence.exists())
        self.publish_error = None
        again = self.auto()
        self.assertEqual(again["result"], "blocked")
        self.assertEqual(self.publications, [3])
        self.assertEqual(len(self.generated), 1)

    def test_unreadable_trust_head_is_a_failure_without_service_changes(self):
        self.query_error = True
        receipt = self.auto()
        self.assertEqual(receipt["result"], "failed")
        self.assertEqual(self.runtime.events, [])

    def test_receipts_are_private_and_hold_no_credentials(self):
        receipt = self.auto()
        directory = self.root / "maintenance/crl-schedule"
        self.assertEqual(directory.stat().st_mode & 0o777, 0o700)
        for path in directory.iterdir():
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
        text = json.dumps(receipt)
        for secret in ("_der", "password", "kek", "key.pem", "original private CA key"):
            self.assertNotIn(secret, text)

    def test_invalid_policy_bounds_are_rejected_before_any_lock(self):
        for threshold, keep in ((0, 4), (7 * 86400, 4), (72 * HOUR, 0), (72 * HOUR, 33)):
            with self.subTest(threshold=threshold, keep=keep):
                with self.assertRaises(ValueError):
                    crl_schedule.run(self.root, threshold, keep, runtime=self.runtime)
        self.assertFalse((self.root / "deploy.lock").exists())


class CrlRetentionTests(CrlFixture):
    def scheduled(self, index, *, state="accepted", inside=True):
        operation = f"20000000-0000-4000-8000-{index:012d}"
        backup = (self.root / "backups" if inside else self.root / "elsewhere") / f"b{index}"
        backup.mkdir(mode=0o700, parents=True)
        (backup / "COMPLETE").write_text("complete")
        journal = self.root / "maintenance/crl" / operation
        journal.mkdir(mode=0o700, parents=True)
        (journal / "journal.json").write_text(json.dumps(
            {"operation_id": operation, "state": state, "backup": {"path": str(backup), "files": {}}}))
        receipts = self.root / "maintenance/crl-schedule"
        receipts.mkdir(mode=0o700, parents=True, exist_ok=True)
        (receipts / f"{1000 + index}-{index:08x}.json").write_text(json.dumps(
            {"checked_at": 1000 + index, "result": "renewed", "operation_id": operation,
             "backup": str(backup)}))
        return backup

    def test_only_scheduled_backups_beyond_the_newest_are_removed(self):
        backups = [self.scheduled(index) for index in range(6)]
        manual = self.root / "backups/manual"
        manual.mkdir(mode=0o700)
        pruned = crl_schedule.prune(self.root, 4)
        self.assertEqual(len(pruned), 2)
        self.assertFalse(backups[0].exists())
        self.assertFalse(backups[1].exists())
        for backup in backups[2:]:
            self.assertTrue(backup.exists())
        self.assertTrue(manual.exists())
        self.assertEqual(crl_schedule.prune(self.root, 4), [])

    def test_backups_without_an_accepted_matching_journal_are_kept(self):
        unaccepted = self.scheduled(0, state="publishing")
        outside = self.scheduled(1, inside=False)
        for index in range(2, 6):
            self.scheduled(index)
        mismatched = self.root / "backups/b2"
        journal = self.root / "maintenance/crl/20000000-0000-4000-8000-000000000002/journal.json"
        record = json.loads(journal.read_text())
        record["backup"]["path"] = str(self.root / "backups/other")
        journal.write_text(json.dumps(record))
        self.assertEqual(crl_schedule.prune(self.root, 3), [])
        for backup in (unaccepted, outside, mismatched):
            self.assertTrue(backup.exists())

    def test_symlinked_backup_is_never_followed(self):
        for index in range(5):
            self.scheduled(index)
        target = self.root / "backups/b0"
        moved = self.root / "elsewhere-target"
        target.rename(moved)
        target.symlink_to(moved)
        crl_schedule.prune(self.root, 4)
        self.assertTrue(moved.exists())


class CrlScheduleUnitTests(unittest.TestCase):
    invocation = {"python": "/opt/cpython/3.12.14/bin/python3.12",
                  "launcher": "/home/qadra/qadra/controller_launcher.py",
                  "root": "/home/qadra/qadra", "inventory_sha256": PIN}

    def test_units_pin_the_exact_launcher_invocation_and_policy(self):
        units = crl_schedule.units(self.invocation, 72, 4)
        self.assertEqual(set(units), set(crl_schedule.UNITS))
        service = units["qadra-crl-renew.service"]
        self.assertIn(f"--inventory-sha256 {PIN} --entrypoint renew_crl.py -- --root /home/qadra/qadra "
                      "auto --threshold-hours 72 --keep-backups 4", service)
        self.assertIn("ExecStart=/opt/cpython/3.12.14/bin/python3.12 -I -B -S "
                      "/home/qadra/qadra/controller_launcher.py --root /home/qadra/qadra", service)
        self.assertIn(f"SuccessExitStatus={crl_schedule.EXIT_BUSY}", service)
        self.assertIn("OnFailure=qadra-crl-alert.service", service)
        for directive in ("TimeoutStartSec=2400", "KillSignal=SIGINT", "KillMode=mixed",
                          "After=qadra-postgres.service qadra-redis.service qadra-api.service"):
            self.assertIn(directive, service)
        timer = units["qadra-crl-renew.timer"]
        for directive in ("OnCalendar=*-*-* 09:00:00 UTC", "Persistent=true", "RandomizedDelaySec="):
            self.assertIn(directive, timer)
        self.assertIn("renew_crl.py -- --root /home/qadra/qadra alert", units["qadra-crl-alert.service"])
        for content in units.values():
            self.assertTrue(content.isascii())

    def test_unsafe_invocation_values_are_rejected(self):
        for field, value in (("inventory_sha256", "C" * 64), ("root", "/home/qadra/q adra"),
                             ("python", "python3"), ("launcher", "/tmp/x;rm"),
                             ("launcher", "/home/qadra/qadra/tools/controller_launcher.py")):
            with self.subTest(field=field):
                with self.assertRaises(ValueError):
                    crl_schedule.units({**self.invocation, field: value}, 72, 4)

    def test_policy_values_are_bounded(self):
        for hours, keep in ((0, 4), (168, 4), (72, 0)):
            with self.subTest(hours=hours, keep=keep):
                with self.assertRaises(ValueError):
                    crl_schedule.units(self.invocation, hours, keep)

    def test_invocation_is_read_from_the_admitted_launcher_command_line(self):
        raw = b"\0".join([b"/opt/cpython/3.12.14/bin/python3.12", b"-I", b"-B", b"-S",
                          b"/home/qadra/qadra/controller_launcher.py", b"--root", b"/home/qadra/qadra",
                          b"--inventory-sha256", PIN.encode(), b"--entrypoint", b"renew_crl.py",
                          b"--", b"--root", b"/home/qadra/qadra", b"schedule-units"]) + b"\0"
        self.assertEqual(crl_schedule.invocation(raw), self.invocation)

    def test_command_lines_without_the_approved_entrypoint_are_rejected(self):
        for raw in (b"python3\0renew_crl.py\0",
                    b"/usr/bin/python3\0-I\0-B\0-S\0/r/controller_launcher.py\0--root\0/r\0"
                    b"--inventory-sha256\0" + PIN.encode() + b"\0--entrypoint\0release.py\0--\0"):
            with self.subTest(raw=raw):
                with self.assertRaises(ValueError):
                    crl_schedule.invocation(raw)


class CrlScheduleEntrypointTests(CrlFixture):
    def command_line(self, root):
        return b"\0".join([b"/opt/cpython/3.12.14/bin/python3.12", b"-I", b"-B", b"-S",
                            str(root / "controller_launcher.py").encode(), b"--root", str(root).encode(),
                            b"--inventory-sha256", PIN.encode(), b"--entrypoint", b"renew_crl.py",
                            b"--", b"--root", str(root).encode(), b"schedule-units"]) + b"\0"

    def test_auto_reports_its_outcome_as_the_process_status(self):
        for result, code in (("busy", crl_schedule.EXIT_BUSY), ("skipped", 0), ("failed", 1)):
            with self.subTest(result=result), \
                    patch.object(crl_schedule, "run", return_value={"result": result}), \
                    patch.object(renew_crl.sys, "argv", ["renew_crl.py", "--root", str(self.root), "auto"]):
                with self.assertRaises(SystemExit) as raised:
                    renew_crl.main()
                self.assertEqual(raised.exception.code, code)

    def test_units_are_installed_from_the_admitted_invocation_and_the_timer_enabled(self):
        calls = []
        directory = self.root / "units"
        result = crl_schedule.install_units(self.root, 72, 4, raw=self.command_line(self.root),
                                            directory=directory,
                                            command=lambda args, check: calls.append(args))
        self.assertEqual(result["inventory_sha256"], PIN)
        self.assertEqual(sorted(path.name for path in directory.iterdir()), sorted(crl_schedule.UNITS))
        self.assertIn(PIN, (directory / "qadra-crl-renew.service").read_text())
        self.assertEqual(calls, [["systemctl", "--user", "daemon-reload"],
                                 ["systemctl", "--user", "enable", "--now", "qadra-crl-renew.timer"]])

    def test_units_for_another_root_are_refused(self):
        other = self.root.parent / "other"
        other.mkdir()
        with self.assertRaises(ValueError):
            crl_schedule.install_units(self.root, 72, 4, raw=self.command_line(other),
                                       directory=self.root / "units", command=lambda *a, **k: None)
        self.assertFalse((self.root / "units").exists())


class CrlAlertTests(CrlFixture):
    def configure(self):
        (self.root / "config/crl-schedule.json").write_text(json.dumps({"alert_to": "operator@example.com"}))
        (self.root / "data/.env").write_text(
            "export RESEND_API_KEY=re_secret_value # provider key\n"
            "REPORT_EMAIL_FROM=\"Qadra <avisos@example.com>\"\n")

    def test_alert_sends_a_generic_summary_without_secrets(self):
        self.configure()
        self.publish_error = "before"
        with patch.object(crl_schedule.time, "time", return_value=self.now):
            crl_schedule.run(self.root, 72 * HOUR, 4, runtime=self.runtime)
        sent = []

        class Response:
            status = 200

            def __enter__(self):
                return self

            def __exit__(self, *_):
                return False

            def read(self, *_):
                return b'{"id": "message"}'

        def urlopen(request, timeout):
            sent.append((request, timeout))
            return Response()

        with patch.object(crl_schedule, "urlopen", side_effect=urlopen), \
                patch.dict(crl_schedule.os.environ, {"MONITOR_INVOCATION_ID": "abc123",
                                                     "MONITOR_SERVICE_RESULT": "exit-code"}):
            crl_schedule.alert(self.root)
        (request, timeout), = sent
        self.assertEqual(request.full_url, "https://api.resend.com/emails")
        self.assertEqual(request.get_header("Authorization"), "Bearer re_secret_value")
        self.assertEqual(request.get_header("Idempotency-key"), "qadra-crl-abc123")
        # The provider's edge rejects the default urllib client signature.
        self.assertEqual(request.get_header("User-agent"), "qadra-crl-alert/1")
        self.assertLessEqual(timeout, 15)
        body = json.loads(request.data)
        self.assertEqual(body["to"], ["operator@example.com"])
        self.assertEqual(body["from"], "Qadra <avisos@example.com>")
        self.assertIn("failed", body["text"])
        self.assertIn("exit-code", body["text"])
        self.assertNotIn("re_secret_value", json.dumps(body))

    def test_alert_without_configuration_fails_visibly(self):
        with self.assertRaises(ValueError):
            crl_schedule.alert(self.root)


if __name__ == "__main__":
    unittest.main()
