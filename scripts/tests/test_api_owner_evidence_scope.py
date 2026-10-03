"""Verify the real PostgreSQL evidence query against multiple public Owners."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from uuid import uuid4


class OwnerEvidenceScope(unittest.TestCase):
    def test_other_owners_cannot_change_the_selected_binding_evidence(self):
        url = os.environ.get("CASE_TEST_DATABASE_URL")
        self.assertTrue(url, "an isolated PostgreSQL database is required")
        schema = "owner_evidence_" + uuid4().hex
        owner, other = str(uuid4()), str(uuid4())
        binding, foreign = str(uuid4()), str(uuid4())

        def sql(statement):
            result = subprocess.run(["psql", url, "-X", "-qAt", "-v", "ON_ERROR_STOP=1",
                                     "-c", statement], capture_output=True, text=True, timeout=15)
            self.assertEqual(result.returncode, 0, "isolated evidence fixture SQL failed")
            return result.stdout

        sql(f'CREATE SCHEMA "{schema}"')
        try:
            with tempfile.TemporaryDirectory(prefix="owner-evidence-") as directory:
                environment = {"TT_OWNER_CERT_WORK": directory, "TT_OWNER_CERT_TOKEN": "fixture",
                               "TT_OWNER_CERT_DATABASE": url, "PGOPTIONS": "-csearch_path=" + schema}
                with patch.dict(os.environ, environment):
                    sql("""CREATE TABLE users(id uuid, email text, role text, active bool,
                             revision bigint, auth_generation bigint);
                        CREATE TABLE owner_certificate_registrations(
                             binding_id uuid, owner_id uuid, audit_sequence bigint);
                        CREATE TABLE owner_certificate_withdrawals(binding_id uuid, audit_sequence bigint);
                        CREATE TABLE audit_events(sequence bigint, action text, actor text, resource text);""")
                    sql(f"""INSERT INTO users VALUES
                        ('{owner}','owner@example.test','owner',true,3,1),
                        ('{other}','other@example.test','owner',true,4,1);
                        INSERT INTO owner_certificate_registrations VALUES
                        ('{binding}','{owner}',10),('{foreign}','{other}',20);
                        INSERT INTO owner_certificate_withdrawals VALUES ('{binding}',11),('{foreign}',21);
                        INSERT INTO audit_events VALUES
                        (10,'identity.owner_certificate_registered','former@example.test','first'),
                        (11,'identity.owner_certificate_withdrawn','former@example.test','first-withdrawal'),
                        (20,'identity.owner_certificate_registered','former@example.test','foreign'),
                        (21,'identity.owner_certificate_withdrawn','former@example.test','foreign-withdrawal');""")
                    source = Path(__file__).resolve().parents[1] / "api_owner_certificate_evidence.py"
                    spec = importlib.util.spec_from_file_location("api_owner_evidence_test", source)
                    helper = importlib.util.module_from_spec(spec)
                    spec.loader.exec_module(helper)
                    before = helper.sql_state(owner)
                    self.assertEqual([r["binding_id"] for r in before["registrations"]], [binding])
                    self.assertEqual([r["binding_id"] for r in before["withdrawals"]], [binding])
                    self.assertEqual([r["sequence"] for r in before["events"]], [10, 11])
                    self.assertEqual(before["account"]["id"], owner)
                    sql(f"DELETE FROM owner_certificate_withdrawals WHERE binding_id='{foreign}';"
                        f"DELETE FROM owner_certificate_registrations WHERE binding_id='{foreign}';"
                        "DELETE FROM audit_events WHERE sequence IN (20,21)")
                    self.assertEqual(helper.sql_state(owner), before)
                    absent = helper.sql_state(str(uuid4()))
                    self.assertEqual(absent, {"registrations": [], "withdrawals": [],
                                              "events": [], "account": None})
        finally:
            sql(f'DROP SCHEMA "{schema}" CASCADE')


if __name__ == "__main__":
    unittest.main()
