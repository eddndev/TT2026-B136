"""Independent canonical intent checks for the disposable HTTP acceptance client."""
import base64
import importlib.util
import os
from pathlib import Path
import sys
import unittest
from unittest.mock import patch
from uuid import UUID


class IndividualSigningIntent(unittest.TestCase):
    def helper(self):
        scripts = Path(__file__).resolve().parents[1]
        values = {"TT_FACT_API_BASE_URL": "http://127.0.0.1:1", "TT_FACT_API_TOKEN": "fixture",
                  "TT_FACT_API_WORK_DIR": "/tmp", "TT_FACT_API_REPO": str(scripts.parent)}
        with patch.dict(os.environ, values), patch.object(sys, "path", [str(scripts), *sys.path]):
            spec = importlib.util.spec_from_file_location("signing_fixture", scripts / "api_individual_signing_support.py")
            helper = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(helper)
        return helper

    def test_client_reconstructs_binding_and_consent_from_exact_visible_references(self):
        helper = self.helper()
        value = {"binding_id": str(UUID(int=4)), "account_id": str(UUID(int=3)),
                 "deployment_id": str(UUID(int=1)), "account_revision": "9007199254740993",
                 "auth_generation": "2", "root_fingerprint": "11" * 32, "trust_revision": 9,
                 "certificate": {"fingerprint": "22" * 32}}
        raw = (b"DOCBIND1\x01\x01" + UUID(int=1).bytes + bytes.fromhex("11" * 32)
               + (9).to_bytes(4, "big") + UUID(int=3).bytes + (9007199254740993).to_bytes(8, "big")
               + (2).to_bytes(8, "big") + UUID(int=4).bytes + bytes.fromhex("22" * 32))
        value["statement_base64"] = base64.b64encode(raw).decode("ascii")
        self.assertEqual(helper.binding_bytes(value), raw)
        consent = {"operation_id": str(UUID(int=5)), "case_id": str(UUID(int=6)),
                   "document_id": str(UUID(int=7)), "version": 8, "document_digest": "33" * 32,
                   "pdf_signature_digest": "44" * 32, "certificate_fingerprint": "22" * 32,
                   **{key: value[key] for key in ["account_id", "binding_id", "account_revision", "auth_generation", "trust_revision"]}}
        canonical = (b"DOCSIGN1" + raw[8:] + UUID(int=5).bytes + UUID(int=6).bytes
                     + UUID(int=7).bytes + (8).to_bytes(4, "big")
                     + bytes.fromhex("33" * 32) + bytes.fromhex("44" * 32))
        consent["consent_base64"] = base64.b64encode(canonical).decode("ascii")
        self.assertEqual(helper.consent_bytes(consent, value), canonical)
        for field, altered in [("case_id", str(UUID(int=9))), ("version", 9),
                               ("document_digest", "55" * 32), ("pdf_signature_digest", "66" * 32),
                               ("account_revision", "9007199254740994")]:
            with self.subTest(field=field), self.assertRaises(AssertionError):
                helper.consent_bytes({**consent, field: altered}, value)


if __name__ == "__main__":
    unittest.main()
