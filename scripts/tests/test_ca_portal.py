import json
from unittest.mock import patch

from ca_issuance_support import IssuanceFixture, OPERATION, OPERATOR, digest, ROOT
from ca_revocation_support import RevocationFixture
import portal


class CaPortalTests(IssuanceFixture):
    def setUp(self):
        super().setUp()
        (self.root / "deploy.lock").touch(mode=0o600)
        self.state = patch("portal.public_state", return_value={"root_fingerprint": digest(ROOT),
            "revoked_serials": [], "revision": 2})
        self.state.start()
        self.addCleanup(self.state.stop)

    def test_preview_pins_root_and_does_not_issue_or_write_journal(self):
        self.executor.describe_request = lambda csr, root: {
            **self.executor.admit(csr, root), "subject": "CN=External holder"}
        result = portal.execute(self.root, OPERATOR, {"action": "preview",
            "csr_pem": self.request["csr_pem"], "purpose": "partner"},
            self.executor, digest(ROOT))
        self.assertEqual(result["root_fingerprint"], digest(ROOT))
        self.assertEqual(result["purpose"], "partner")
        self.assertEqual(self.executor.signatures, [])
        self.assertFalse(self.operation.exists())
        with self.assertRaises(ValueError):
            portal.execute(self.root, OPERATOR, {"action": "preview",
                "csr_pem": self.request["csr_pem"], "purpose": "partner"},
                self.executor, "0" * 64)

    def test_issue_history_and_resume_preserve_actor_and_exact_receipt(self):
        command = {"action": "issue", "operation_id": OPERATION,
                   **{k: v for k, v in self.request.items() if k != "operator_id"}}
        first = portal.execute(self.root, OPERATOR, command, self.executor, digest(ROOT))
        repeated = portal.execute(self.root, OPERATOR,
            {"action": "resume-issue", "operation_id": OPERATION}, self.executor, digest(ROOT))
        self.assertEqual(first, repeated)
        self.assertEqual(first["operator_id"], OPERATOR)
        history = portal.execute(self.root, OPERATOR,
            {"action": "history", "offset": 0, "limit": 10}, self.executor, digest(ROOT))
        self.assertEqual(history["total"], 1)
        self.assertEqual(history["items"][0]["state"], "completed")
        self.assertNotIn("csr_pem", json.dumps(history))
        self.assertNotIn("certificate_pem", json.dumps(history))
        self.assertEqual(self.executor.signatures, ["1000"])

    def test_actor_paths_commands_and_unbounded_pages_are_rejected(self):
        for command in [
            {"action": "shell", "command": "anything"},
            {"action": "history", "offset": 0, "limit": 101},
            {"action": "history", "offset": -1, "limit": 10},
            {"action": "history", "offset": 0, "limit": 10, "operator_id": OPERATOR},
        ]:
            with self.assertRaises(ValueError):
                portal.execute(self.root, OPERATOR, command, self.executor, digest(ROOT))
        with self.assertRaises(ValueError):
            portal.execute(self.root, "../../operator", {"action": "status"}, self.executor, digest(ROOT))

    def test_inventory_marks_only_persisted_trust_as_published(self):
        row = {"serial": "1000", "local_status": "revoked", "published_status": "unknown"}
        state = {"root_fingerprint": digest(ROOT), "revoked_serials": [], "revision": 2}
        with patch("portal.inventory.list_certificates", return_value=[row]), \
                patch("portal.public_state", return_value=state):
            result = portal.execute(self.root, OPERATOR,
                {"action": "inventory", "offset": 0, "limit": 10}, self.executor, digest(ROOT))
            self.assertEqual(result["items"][0]["published_status"], "pending")
            state["revoked_serials"] = ["1000"]
            result = portal.execute(self.root, OPERATOR,
                {"action": "inventory", "offset": 0, "limit": 10}, self.executor, digest(ROOT))
            self.assertEqual(result["items"][0]["published_status"], "revoked")


class CaPortalRevocationTests(RevocationFixture):
    def test_revocation_and_recovery_keep_receipt_in_history_without_prior_portal_issuance(self):
        with patch("portal.public_state", return_value={"root_fingerprint": digest(ROOT)}):
            command = {"action": "revoke", "operation_id": OPERATION,
                       **{k: v for k, v in self.revoke_request.items() if k != "operator_id"}}
            receipt = portal.execute(self.root, OPERATOR, command, self.executor, digest(ROOT))
            repeated = portal.execute(self.root, OPERATOR, {"action": "resume-revoke",
                "operation_id": OPERATION}, self.executor, digest(ROOT))
        self.assertEqual(receipt, repeated)
        self.assertEqual(self.executor.calls, 1)
        rows = portal.history(self.root)
        self.assertEqual(len(rows), 1)
        self.assertEqual((rows[0]["kind"], rows[0]["state"]), ("revoke", "completed"))
        self.assertEqual(rows[0]["operator_id"], OPERATOR)

    def test_unreconciled_published_root_blocks_new_revocation(self):
        with patch("portal.public_state", side_effect=ValueError("root mismatch")):
            with self.assertRaises(ValueError):
                portal.execute(self.root, OPERATOR, {"action": "revoke", "operation_id": OPERATION,
                    **{k: v for k, v in self.revoke_request.items() if k != "operator_id"}},
                    self.executor, digest(ROOT))
        self.assertEqual(self.executor.calls, 0)
        self.assertFalse(self.revocation.exists())
