"""The authenticated portal maps publication to the existing maintenance flow."""
from ca_issuance_support import OPERATION, OPERATOR
from test_ca_publication import PublicationFixture
import portal


class PortalPublicationTests(PublicationFixture):
    def test_publication_history_and_retry_keep_original_actor_and_revision(self):
        pin = self.request["expected_root_fingerprint"]
        receipt = portal.execute(self.root, OPERATOR, {"action": "renew-crl",
            "operation_id": OPERATION, "expected_revision": 3,
            "acknowledge_maintenance": True}, None, pin)
        self.assert_receipt(receipt)
        repeated = portal.execute(self.root, OPERATOR, {"action": "resume-crl",
            "operation_id": OPERATION}, None, pin)
        self.assertEqual(repeated, receipt)
        rows = portal.history(self.root)
        self.assertEqual(len(rows), 1)
        self.assertEqual((rows[0]["kind"], rows[0]["state"]), ("publication", "completed"))
        self.assertEqual(rows[0]["operator_id"], OPERATOR)
        self.assertEqual(rows[0]["revision"], 4)
        self.assertEqual(len(self.generated), 1)

    def test_recovery_does_not_change_adopted_root(self):
        self.portal_publish()
        with self.assertRaises(ValueError):
            portal.execute(self.root, OPERATOR, {"action": "resume-crl",
                "operation_id": OPERATION}, None, "0" * 64)
        self.assertEqual(len(self.generated), 1)
