"""Status reports observed credential validity without implying renewal ran."""
import json
import unittest
from unittest.mock import patch

from test_deploy_crl_support import CrlFixture, renew_crl


class CrlStatusTests(CrlFixture):
    def test_status_reports_the_persisted_validity_at_one_observed_instant(self):
        with patch.object(renew_crl.time, "time", return_value=self.now):
            result = renew_crl.status(self.root)
        self.assertEqual(result, {
            "deployment_id": self.head["deployment_id"], "revision": 3,
            "crl_number": 4096, "checked_at": self.now,
            "valid_from": self.head["valid_from"], "valid_until": self.head["valid_until"],
            "expires_in_seconds": 86400, "valid_now": True, "maintenance_pending": False,
        })
        self.assertEqual(self.runtime.events, [])
        self.assertEqual(self.publications, [])
        self.assertEqual(self.generated, [])

    def test_expired_or_not_yet_valid_material_is_not_reported_as_currently_valid(self):
        for first, last, remaining in ((self.now - 86400, self.now - 1, 0),
                                       (self.now + 3600, self.now + 86400, 86400)):
            with self.subTest(first=first, last=last):
                self.head.update(valid_from=first, valid_until=last)
                with patch.object(renew_crl.time, "time", return_value=self.now):
                    result = renew_crl.status(self.root)
                self.assertFalse(result["valid_now"])
                self.assertEqual(result["expires_in_seconds"], remaining)
                self.assertEqual(result["checked_at"], self.now)
        self.assertEqual(self.publications, [])
        self.assertEqual(self.runtime.events, [])

    def test_status_includes_both_persisted_validity_boundaries(self):
        self.head.update(valid_from=self.now, valid_until=self.now + 1)
        for instant in (self.now, self.now + 1):
            with self.subTest(instant=instant), patch.object(renew_crl.time, "time", return_value=instant):
                self.assertTrue(renew_crl.status(self.root)["valid_now"])

    def test_pending_maintenance_is_reported_even_when_the_persisted_crl_is_valid(self):
        operation = "10000000-0000-4000-8000-000000000001"
        self.fence.write_text(json.dumps({"operation_id": operation}))
        with patch.object(renew_crl.time, "time", return_value=self.now):
            result = renew_crl.status(self.root)
        self.assertTrue(result["maintenance_pending"])
        self.assertEqual(result["operation_id"], operation)
        self.assertTrue(result["valid_now"])
        self.assertEqual(self.runtime.events, [])
        self.assertEqual(self.publications, [])


if __name__ == "__main__":
    unittest.main()
