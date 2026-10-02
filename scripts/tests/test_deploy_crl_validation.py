"""Renewal checks the exact original trust and rejects unsafe candidate CRLs."""
import copy
import unittest

from test_deploy_crl_support import CrlFixture, fingerprint


class CrlValidationTests(CrlFixture):
    def assert_candidate_rejected(self):
        with self.assertRaises((ValueError, RuntimeError)):
            self.renew()
        self.assertEqual(self.publications, [])
        self.assertEqual(self.active_crl.read_bytes(), b"original CRL")
        self.assertEqual(self.counter.read_bytes(), b"1001\n")
        self.assertFalse(self.fence.exists())
        self.assert_keys_preserved()

    def test_a_candidate_from_another_root_is_rejected(self):
        self.candidate["root_der"] = b"different root".hex()
        self.candidate["root_fingerprint"] = fingerprint(b"different root")
        self.assert_candidate_rejected()

    def test_candidate_digest_must_describe_the_inspected_der(self):
        self.candidate["crl_digest"] = fingerprint(b"unrelated CRL")
        self.assert_candidate_rejected()

    def test_candidate_number_must_increase_without_exceeding_persisted_u64(self):
        for number in (0, 4095, 4096, 2**64):
            with self.subTest(number=number):
                self.candidate["crl_number"] = number
                self.assert_candidate_rejected()

    def test_candidate_must_be_current_and_not_predate_the_original_crl(self):
        original = copy.deepcopy(self.candidate)
        cases = (
            {"crl_this_update": self.baseline["crl_this_update"] - 1},
            {"crl_this_update": self.now + 3600, "valid_from": self.now + 3600},
            {"crl_next_update": self.now - 1, "valid_until": self.now - 1},
            {"valid_until": self.now - 1},
            {"valid_from": self.now + 3600},
        )
        for changed in cases:
            with self.subTest(changed=changed):
                self.candidate = {**original, **changed}
                self.assert_candidate_rejected()

    def test_renewal_cannot_remove_an_existing_revocation(self):
        self.candidate["revoked_serials"] = []
        self.assert_candidate_rejected()

    def test_retained_revocations_and_counter_gaps_are_accepted(self):
        self.counter.write_text("1010\n")
        self.candidate["crl_number"] = 4112
        self.candidate["revoked_serials"].append("1001")
        self.assertEqual(self.renew()["revision"], 4)
        self.assertEqual(int(self.counter.read_text(), 16), 4113)
        self.assertEqual(self.head["revoked_serials"], ["1000", "1001"])

    def test_preexisting_expired_crl_can_be_renewed_with_a_fresh_candidate(self):
        self.head["crl_next_update"] = self.now - 1
        self.head["valid_until"] = self.now - 1
        self.baseline = copy.deepcopy(self.head)
        self.assertEqual(self.renew()["revision"], 4)
        self.assertTrue(self.runtime.running)
        self.assert_keys_preserved()

    def test_local_original_must_match_both_der_values_in_persisted_trust(self):
        original = copy.deepcopy(self.head)
        for key, value in (("root_der", b"other root".hex()),
                           ("crl_der", b"other CRL".hex())):
            with self.subTest(key=key):
                self.head = {**original, key: value}
                with self.assertRaises((ValueError, RuntimeError)):
                    self.renew()
                self.assertEqual(self.generated, [])
                self.assertEqual(self.publications, [])
                self.assertEqual(self.runtime.events, [])

    def test_counter_must_be_valid_and_ahead_of_the_persisted_number(self):
        for counter in ("not-hex\n", "1000\n", "0FFF\n", "-1\n"):
            with self.subTest(counter=counter):
                self.counter.write_text(counter)
                with self.assertRaises((ValueError, RuntimeError)):
                    self.renew()
                self.assertEqual(self.generated, [])
                self.assertEqual(self.publications, [])
                self.assertEqual(self.runtime.events, [])


if __name__ == "__main__":
    unittest.main()
