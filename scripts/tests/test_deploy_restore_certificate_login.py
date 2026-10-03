"""Remove pre-proof captures without admitting restored authentication state."""
import copy
import unittest

from deploy_restore_certificate_login_support import (
    CAPTURE_PATTERN, CAPTURE_PREFIX, CONTROL_KEYS, PATTERNS, CertificateInvalidationFixture,
)


class CertificateLoginRestoreTests(CertificateInvalidationFixture):
    def test_all_authentication_namespaces_are_scanned_before_and_after_selective_removal(self):
        before = copy.deepcopy(self.redis.values)
        result = self.invoke()
        self.assertEqual(result, {"sessions_removed": 2, "challenges_removed": 1,
                                  "certificate_logins_removed": 2})
        self.assertNotIn("certificate-logins_removed", result)
        expected = self.preserved(before)
        self.assertTrue(self.redis.values == expected, "retained values, types or expiries changed")
        first = next(index for index, call in enumerate(self.redis.calls) if call[0] == "DEL")
        last = max(index for index, call in enumerate(self.redis.calls) if call[0] == "DEL")
        for pattern in PATTERNS:
            self.assertTrue(any(call[0] == "SCAN" and pattern in call for call in self.redis.calls[:first]),
                            "deletion preceded a complete authentication inventory")
            self.assertTrue(any(call[0] == "SCAN" and pattern in call for call in self.redis.calls[last + 1:]),
                            "success lacked an empty final authentication inventory")
        self.assertEqual(self.invoke(), {"sessions_removed": 0, "challenges_removed": 0,
                                        "certificate_logins_removed": 0})
        self.assertTrue(self.redis.values == expected)

    def test_invalid_capture_scan_and_shared_limits_reject_before_any_delete(self):
        changes = [
            ("short-digest", {"capture_page": ["0", [CAPTURE_PREFIX + "a" * 63]]}, {}),
            ("upper-digest", {"capture_page": ["0", [CAPTURE_PREFIX + "A" * 64]]}, {}),
            ("nested-key", {"capture_page": ["0", [CAPTURE_PREFIX + "a" * 64 + ":extra"]]}, {}),
            ("foreign-control", {"capture_page": ["0", [CONTROL_KEYS[0]]]}, {}),
            ("invalid-cursor", {"capture_page": ["18446744073709551616", []]}, {}),
            ("unfinished", {"capture_endless": True}, {}),
            ("shared-key-limit", {}, {"max_keys": 4}),
            ("reserved-final-scans", {}, {"max_scan_calls": 7}),
        ]
        for label, edits, options in changes:
            with self.subTest(reason=label):
                self.reset_commands()
                for name, value in edits.items():
                    setattr(self.redis, name, value)
                before = copy.deepcopy(self.redis.values)
                self.rejection(**options)
                self.assertEqual(self.redis.delete_calls, 0)
                self.assertTrue(self.redis.values == before, "failed preflight changed Redis state")
                self.assertLessEqual(self.redis.scan_calls, options.get("max_scan_calls", 12))

    def test_capture_appearing_during_deletion_prevents_receipt_until_explicit_retry(self):
        before = copy.deepcopy(self.redis.values)
        self.redis.extra = CAPTURE_PREFIX + "f" * 64
        self.redis.appear_after_delete = True
        self.rejection()
        self.assertIn(self.redis.extra, self.redis.values)
        self.assertTrue(self.preserved(self.redis.values) == self.preserved(before))
        self.assertEqual(self.invoke(), {"sessions_removed": 0, "challenges_removed": 0,
                                        "certificate_logins_removed": 1})
        self.assertTrue(self.redis.values == self.preserved(before))

    def test_transport_failure_in_capture_removal_never_claims_success_or_erases_controls(self):
        before = copy.deepcopy(self.redis.values)
        self.redis.fail_capture_delete = True
        self.rejection()
        self.assertTrue(self.preserved(self.redis.values) == self.preserved(before))
        self.assertEqual(sum(key.startswith(CAPTURE_PREFIX) for key in self.redis.values), 2)
        self.assertEqual(self.invoke(), {"sessions_removed": 0, "challenges_removed": 0,
                                        "certificate_logins_removed": 2})
        self.assertTrue(self.redis.values == self.preserved(before))

    def test_empty_capture_namespace_still_requires_initial_and_final_scans_and_zero_counter(self):
        self.redis.values = self.preserved(self.redis.values)
        before = copy.deepcopy(self.redis.values)
        result = self.invoke()
        self.assertEqual(result, {"sessions_removed": 0, "challenges_removed": 0,
                                  "certificate_logins_removed": 0})
        self.assertEqual(sum(call[0] == "SCAN" and CAPTURE_PATTERN in call for call in self.redis.calls), 2)
        self.assertEqual(self.redis.delete_calls, 0)
        self.assertTrue(self.redis.values == before)


if __name__ == "__main__":
    unittest.main()
