"""Exact restore compatibility is read-only and cannot authorize publication."""
import copy
import json

from deploy_backup_manifest_support import NAME
from deploy_restore_admission_support import AdmissionFixture, fingerprint


class RestoreAdmissionTests(AdmissionFixture):
    def test_exact_compatibility_returns_saved_evidence_without_changing_a_pending_fence(self):
        before = self.snapshot()
        result = self.invoke()
        self.assertEqual(result, {
            "backup_id": self.descriptor["backup"]["id"],
            "descriptor_sha256": self.pin,
            "audit_predecessor": self.descriptor["audit_predecessor"],
        })
        self.assertTrue(self.snapshot() == before, "compatibility changed deployment files")

    def test_unpinned_mixed_legacy_and_uninitialized_captures_are_not_adopted(self):
        self.rejected(expected="0" * 64)
        original = {name: (self.backup / name).read_bytes()
                    for name in self.descriptor["backup"]["files"]}
        for name in ("database.dump", "private-state.tar.gz", "COMPLETE", NAME):
            with self.subTest(changed=name):
                (self.backup / name).write_bytes(original[name] + b"changed")
                self.rejected()
                (self.backup / name).write_bytes(original[name])
        (self.backup / NAME).unlink()
        self.rejected()
        (self.backup / NAME).write_bytes(original[NAME])
        (self.backup / NAME).chmod(0o600)
        manifest = json.loads(original[NAME])
        manifest["source"].update(initialized=False, schema=None, release=None)
        self.fixture.write_json(self.backup / NAME, manifest)
        self.descriptor["backup"]["files"] = self.files()
        self.write_descriptor()
        self.rejected()

    def test_installed_files_migrations_controller_and_symlink_ancestors_are_verified(self):
        paths = [self.target / "bin/despacho-cli", self.target / "migrations/0001_fixture.sql",
                 self.controller / "runtime.py", self.target / "release.json"]
        for path in paths:
            with self.subTest(changed=path.name):
                value = path.read_bytes()
                path.write_bytes(value + b"\nchanged")
                self.rejected()
                path.write_bytes(value)
        extra = self.target / "unrecorded-file"
        extra.write_bytes(b"unrecorded")
        self.rejected()
        extra.unlink()
        binary_directory = self.target / "bin"
        redirected = self.root / "receipts/redirected-bin"
        binary_directory.rename(redirected)
        binary_directory.symlink_to(redirected, target_is_directory=True)
        self.rejected()
        binary_directory.unlink()
        redirected.rename(binary_directory)
        outside = self.root / "releases/escaped"
        outside.write_bytes(b"outside the selected release")
        declared = json.loads(self.fixture.release_file.read_bytes())
        declared["files"]["../escaped"] = fingerprint(outside.read_bytes())["sha256"]
        self.fixture.write_json(self.fixture.release_file, declared)
        manifest = self.fixture.document(self.backup)
        manifest["source"]["release"]["manifest_sha256"] = fingerprint(
            self.fixture.release_file.read_bytes())["sha256"]
        self.fixture.write_json(self.backup / NAME, manifest)
        self.descriptor["release"] = copy.deepcopy(manifest["source"]["release"])
        self.refresh_binding()
        self.rejected()

    def test_different_tool_engine_database_role_or_persistence_facts_are_rejected(self):
        before = copy.deepcopy(self.observed)
        candidates = [
            ("postgres", "server_version_num", 160016),
            ("postgres", "pg_dump_version", "16.16"),
            ("postgres", "database", "qadra_other"),
            ("redis", "engine", "valkey"),
            ("redis", "rdb_version", 11),
            ("redis", "appendonly", False),
        ]
        for group, key, value in candidates:
            with self.subTest(group=group, field=key):
                self.observed = copy.deepcopy(before)
                self.observed[group][key] = value
                self.rejected()
        self.observed = copy.deepcopy(before)
        self.observed["postgres"]["tools"]["pg_restore"]["sha256"] = "e" * 64
        self.rejected()
        self.observed = copy.deepcopy(before)
        self.observed["postgres"]["roles"]["qadra_runtime"]["member_of"] = ["qadra_admin"]
        self.rejected()
        self.observed = copy.deepcopy(before)
        for facts in (self.observed, self.descriptor):
            facts["postgres"]["roles"]["qadra_runtime"]["superuser"] = True
        self.write_descriptor()
        self.rejected()

    def test_audit_predecessor_is_explicit_bounded_and_never_inferred(self):
        for predecessor in (None, {"sequence": 0, "head": "c" * 64}):
            with self.subTest(empty=predecessor is None):
                self.descriptor["audit_predecessor"] = predecessor
                self.write_descriptor()
                before = self.snapshot()
                self.assertEqual(self.invoke()["audit_predecessor"], predecessor)
                self.assertTrue(self.snapshot() == before)
        candidates = [{"sequence": value, "head": "c" * 64} for value in (-1, 2**63, True)]
        candidates += [{"sequence": 0, "head": "C" * 64}, {"unknown": True}]
        for value in candidates:
            with self.subTest(value=value):
                self.descriptor["audit_predecessor"] = value
                self.write_descriptor()
                self.rejected()
        del self.descriptor["audit_predecessor"]
        self.write_descriptor()
        self.rejected()

    def test_coherently_rebound_wrong_container_headers_and_pending_crl_are_rejected(self):
        for name, replacement in (("database.dump", b"not-a-custom-dump"),
                                  ("redis.rdb", b"REDIS0011-fake-content")):
            with self.subTest(payload=name):
                path = self.backup / name
                before = path.read_bytes()
                path.write_bytes(replacement)
                self.refresh_binding()
                self.rejected()
                path.write_bytes(before)
                self.refresh_binding()
        crl = self.root / "config/crl-maintenance.json"
        crl.write_bytes(b"uncertain CRL transition")
        crl.chmod(0o600)
        self.rejected()

    def test_descriptor_json_byte_permission_and_link_boundaries_preserve_all_inputs(self):
        original = self.descriptor_path.read_bytes()
        baseline = copy.deepcopy(self.descriptor)
        self.descriptor_path.chmod(0o644)
        self.rejected()
        self.descriptor_path.chmod(0o600)
        outside = self.root / "receipts/separate-descriptor.json"
        outside.write_bytes(original)
        outside.chmod(0o600)
        self.descriptor_path.unlink()
        self.descriptor_path.symlink_to(outside)
        self.rejected()
        self.descriptor_path.unlink()
        self.descriptor_path.write_bytes(original)
        self.descriptor_path.chmod(0o600)
        for encoded in (b" " * (64 * 1024 + 1),
                        original.replace(b'"version": 1', b'"version": 1, "version": 1')):
            with self.subTest(encoded_bytes=len(encoded)):
                self.descriptor_path.write_bytes(encoded)
                self.pin = fingerprint(encoded)["sha256"]
                self.rejected()
        mutations = [lambda value: value.update(version=2),
                     lambda value: value.update(unexpected=True),
                     lambda value: value["postgres"]["tools"]["server"].update(bytes=0),
                     lambda value: value["postgres"]["tools"]["server"].update(bytes=512 * 1024**2 + 1),
                     lambda value: value["postgres"].update(server_version_num=True)]
        for index, mutate in enumerate(mutations):
            with self.subTest(shape=index):
                self.descriptor = copy.deepcopy(baseline)
                mutate(self.descriptor)
                self.write_descriptor()
                self.rejected()


if __name__ == "__main__":
    import unittest
    unittest.main()
