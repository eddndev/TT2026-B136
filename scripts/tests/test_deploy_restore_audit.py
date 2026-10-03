"""Preserve SQL audit timestamp text and clean the verifier's private sidecar."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch

from deploy_restore_capture_support import CaptureFixture
import crl_journal
import restore_capture
import restore_catalog
import runtime


class RestoreAuditTests(CaptureFixture):
    def rfc3339_rows(self):
        return [{**self.audit[0], "sequence": index, "timestamp": value}
                for index, value in enumerate(("2026-10-02T17:04:05Z",
                    "2026-10-02T17:04:05.123456Z", "2026-10-02T17:04:05.000000001Z"))]

    def verify(self):
        return restore_capture.verified_audit(
            self.root, self.tools, self.targets["postgres"], self.target)

    def test_rfc3339_sql_text_reaches_final_verifier_without_precision_loss(self):
        self.audit = self.rfc3339_rows()
        with crl_journal.locked(self.root), \
                patch.object(restore_catalog, "run", side_effect=self.execute), \
                patch.object(restore_capture, "run", side_effect=self.execute):
            rows, encoded = restore_catalog.audit(
                self.tools, runtime.environment(self.root, admin=True), "public")
            self.assertTrue(rows == self.audit, "SQL audit values were changed")
            decoded = [json.loads(line) for line in encoded.splitlines()]
            self.assertTrue(decoded == self.audit, "JSONL changed timestamp text or precision")
            verified, head = self.verify()
            self.assertTrue(verified == self.audit)
            self.assertEqual(head, {"sequence": 2, "head": self.audit[-1]["chain"]})
            self.assertEqual(len(self.verified_exports), 1, "final verifier was bypassed")
            self.audit_valid = False
            with self.assertRaises(ValueError):
                self.verify()
            self.assertEqual(len(self.verified_exports), 2)
        self.assertTrue(all(not path.exists() for path in self.verified_exports))

    def test_verifier_sidecar_is_removed_on_success_and_rejection(self):
        self.audit = self.rfc3339_rows()
        encoded = b"".join((json.dumps(row) + "\n").encode() for row in self.audit)
        sidecars = []
        def verifier(arguments, **kwargs):
            path = Path(kwargs["env"]["AUDIT_LOG_PATH"] + ".lock")
            path.write_bytes(b"")
            path.chmod(0o600)
            sidecars.append(path)
            return self.execute(arguments, **kwargs)
        # This case isolates verifier cleanup from the separately tested SQL parser.
        with patch.object(restore_catalog, "audit", return_value=(self.audit, encoded)), \
                patch.object(restore_capture, "run", side_effect=verifier):
            for valid in (True, False):
                with self.subTest(verifier_accepts=valid):
                    self.audit_valid = valid
                    if valid:
                        self.verify()
                    else:
                        with self.assertRaises(ValueError):
                            self.verify()
                    self.assertTrue(all(not path.exists() for path in self.verified_exports),
                                    "private audit export survived verification")
                    self.assertTrue(all(not path.exists() for path in sidecars),
                                    "private audit lock sidecar survived verification")


if __name__ == "__main__":
    unittest.main()
