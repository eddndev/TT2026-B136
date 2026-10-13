"""Read-only certificate inventory, independent of CRL publication claims."""
import json
import unittest

from ca_issuance_support import IssuanceFixture, ROOT, digest, encoded, journal
import inventory


class PublicInspector:
    def __init__(self, owner):
        self.owner = owner
        self.inspected = []

    def describe_certificate(self, certificate_pem, root_pem):
        self.owner.assertEqual(root_pem, ROOT)
        facts = json.loads(certificate_pem)
        self.inspected.append(certificate_pem)
        # Whitespace outside this synthetic certificate does not change its DER.
        return {**facts, "fingerprint": digest(encoded(facts))}


class CaInventoryTests(IssuanceFixture):
    def setUp(self):
        super().setUp()
        (self.root / "deploy.lock").write_bytes(b"")
        self.executor = PublicInspector(self)

    def public_certificate(self, **overrides):
        facts = {"subject": "CN=Declared Holder", "not_before": 100, "not_after": 200}
        return self.certificate(**{**facts, **overrides})

    def add_certificate(self, serial="1000", status="V", locations=("newcerts", "certs"),
                        **overrides):
        certificate = self.public_certificate(serial=serial, **overrides)
        for location in locations:
            name = serial + ".pem" if location == "newcerts" else serial + ".crt.pem"
            (self.ca / location / name).write_bytes(certificate)
        row = self.index_row(serial)
        if status == "R":
            row = row.replace("V\t", "R\t", 1).replace("Z\t\t", "Z\t260101000000Z\t", 1)
        else:
            row = status + row[1:]
        with (self.ca / "index.txt").open("a", encoding="ascii") as stream:
            stream.write(row)
        return certificate

    def inventory(self, now=150):
        return inventory.list_certificates(self.root, self.executor, now_unix=now)

    def download(self, serial="1000", fingerprint=None):
        return inventory.download_certificate(
            self.root, serial, fingerprint or digest(self.public_certificate()), self.executor)

    def test_inventory_uses_signed_metadata_and_separates_local_publication_validity(self):
        self.add_certificate("1002", "R")
        documentary = self.add_certificate("1000", purpose="document-signing")
        self.add_certificate("1001", "E", not_after=149)
        original = self.state()
        result = self.inventory()
        self.assertEqual([row["serial"] for row in result], ["1000", "1001", "1002"])
        self.assertEqual(result[0], {
            "serial": "1000", "subject": "CN=Declared Holder",
            "purpose": "document-signing", "not_before": 100, "not_after": 200,
            "fingerprint": digest(documentary), "root_fingerprint": digest(ROOT),
            "validity": "valid", "local_status": "issued", "published_status": "unknown",
        })
        self.assertEqual((result[1]["local_status"], result[1]["validity"]), ("expired", "expired"))
        self.assertEqual((result[2]["local_status"], result[2]["validity"]), ("revoked", "valid"))
        self.assertTrue(all(row["published_status"] == "unknown" for row in result))
        self.assertEqual(self.state(), original)

    def test_signed_validity_keeps_both_inclusive_boundaries(self):
        self.add_certificate()
        for now, expected in ((99, "not-yet-valid"), (100, "valid"), (200, "valid"), (201, "expired")):
            with self.subTest(now=now):
                row = self.inventory(now)[0]
                self.assertEqual(row["validity"], expected)
                self.assertEqual(row["local_status"], "issued")

    def test_empty_inventory_has_no_side_effect_or_executor_call(self):
        original = self.state()
        self.assertEqual(self.inventory(), [])
        self.assertEqual(self.executor.inspected, [])
        self.assertEqual(self.state(), original)

    def test_legacy_named_certificate_directory_without_newcerts_is_supported(self):
        certificate = self.add_certificate(locations=("certs",))
        (self.ca / "certs/1000.crt.pem").rename(self.ca / "certs/legacy-holder.crt.pem")
        (self.ca / "newcerts").rmdir()
        self.assertEqual(self.inventory()[0]["serial"], "1000")
        self.assertEqual(self.download(), certificate)

    def test_serial_archive_without_named_certificate_directory_is_supported(self):
        certificate = self.add_certificate(locations=("newcerts",))
        (self.ca / "certs").rmdir()
        self.assertEqual(self.inventory()[0]["serial"], "1000")
        self.assertEqual(self.download(), certificate)

    def test_duplicate_der_is_one_entry_and_archive_download_preserves_exact_bytes(self):
        certificate = self.add_certificate()
        (self.ca / "newcerts/1000.pem").write_bytes(b"\n" + certificate)
        self.assertEqual(len(self.inventory()), 1)
        self.assertEqual(self.download("00001000"), b"\n" + certificate)

    def test_old_certificate_remains_listed_after_same_subject_gets_another_serial(self):
        first = self.add_certificate("1000", "R")
        second = self.add_certificate("1001")
        self.assertEqual(len(self.inventory()), 2)
        self.assertEqual(self.download("1000", digest(first)), first)
        self.assertEqual(self.download("1001", digest(second)), second)

    def test_download_requires_matching_serial_and_der_fingerprint(self):
        self.add_certificate()
        for serial, fingerprint in (("../1000", "0" * 64), ("1000.pem", "0" * 64),
                                     ("0", "0" * 64), ("1000", "A" * 64),
                                     ("1000", "0" * 64), ("1000", "x" * 64)):
            with self.subTest(serial=serial, fingerprint=fingerprint):
                with self.assertRaises(ValueError):
                    self.download(serial, fingerprint)
        with self.assertRaises(LookupError):
            self.download("1001", "0" * 64)

    def test_inventory_never_traverses_private_material_or_request_files(self):
        self.add_certificate()
        for directory in ("private", "csr"):
            (self.ca / directory).mkdir()
            (self.ca / directory / "1000.pem").write_bytes(b"not public certificate material")
        (self.ca / "certs/holder.key.pem").write_bytes(b"not public certificate material")
        self.assertEqual(len(self.inventory()), 1)
        self.assertTrue(all(b"not public" not in value for value in self.executor.inspected))

    def test_missing_index_certificate_cannot_be_inferred(self):
        self.add_certificate(locations=())
        with self.assertRaises(ValueError):
            self.inventory()

    def test_certificate_without_index_entry_cannot_get_an_invented_local_status(self):
        self.add_certificate()
        (self.ca / "index.txt").write_bytes(b"")
        with self.assertRaises(ValueError):
            self.inventory()

    def test_conflicting_certificate_for_one_serial_rejects_inventory_and_download(self):
        self.add_certificate()
        (self.ca / "certs/1000.crt.pem").write_bytes(
            self.public_certificate(spki_sha256=digest(b"another public key")))
        for read in (self.inventory, self.download):
            with self.assertRaises(ValueError):
                read()

    def test_serial_archive_filename_must_match_signed_serial(self):
        self.add_certificate()
        (self.ca / "newcerts/1000.pem").rename(self.ca / "newcerts/1001.pem")
        with self.assertRaises(ValueError):
            self.inventory()

    def test_public_files_cannot_be_symlinks_or_exceed_certificate_bound(self):
        self.add_certificate(locations=("certs",))
        path = self.ca / "newcerts/1000.pem"
        path.symlink_to(self.ca / "certs/1000.crt.pem")
        with self.assertRaises((ValueError, OSError)):
            self.inventory()
        path.unlink()
        path.write_bytes(b"x" * 16385)
        with self.assertRaises(ValueError):
            self.inventory()

    def test_public_directory_cannot_be_a_symlink(self):
        self.add_certificate(locations=("newcerts",))
        (self.ca / "certs").rmdir()
        (self.ca / "certs").symlink_to(self.ca / "newcerts", target_is_directory=True)
        with self.assertRaises(ValueError):
            self.inventory()

    def test_inspector_metadata_is_bounded_and_does_not_allow_extra_fields(self):
        self.add_certificate(locations=("newcerts",))
        for override in ({"not_after": True}, {"not_after": 99}, {"subject": ""},
                         {"subject": "x" * 4097}, {"purpose": "root"},
                         {"root_fingerprint": "x" * 64}, {"private_key": "excluded"}):
            with self.subTest(override=override):
                certificate = self.public_certificate(**override)
                (self.ca / "newcerts/1000.pem").write_bytes(certificate)
                with self.assertRaises(ValueError):
                    self.inventory()

    def test_inspector_signature_rejection_is_not_converted_into_a_valid_entry(self):
        self.add_certificate()
        def reject(certificate_pem, root_pem):
            raise ValueError("certificate signature does not match the configured root")
        self.executor.describe_certificate = reject
        with self.assertRaises(ValueError):
            self.inventory()

    def test_read_lock_prevents_a_concurrent_ca_mutation(self):
        self.add_certificate()
        with journal.locked(self.root):
            with self.assertRaises(BlockingIOError):
                self.inventory()
            with self.assertRaises(BlockingIOError):
                self.download()
        self.assertEqual(self.executor.inspected, [])

    def test_read_does_not_create_a_missing_deploy_lock(self):
        (self.root / "deploy.lock").unlink()
        with self.assertRaises(FileNotFoundError):
            self.inventory()
        self.assertFalse((self.root / "deploy.lock").exists())

    def test_now_must_be_an_injected_integer(self):
        for now in (None, True, 150.5, "150"):
            with self.subTest(now=now):
                with self.assertRaises(ValueError):
                    self.inventory(now)


if __name__ == "__main__":
    unittest.main()
