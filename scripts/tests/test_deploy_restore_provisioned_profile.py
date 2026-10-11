"""Capture the provisioned Redis profile without accepting another destination."""
import copy
import json
import unittest
from unittest.mock import patch

from deploy_restore_capture_support import CaptureFixture
import host
import restore_admission


class ProvisionedRestoreProfileTests(CaptureFixture):
    def setUp(self):
        super().setUp()
        self.settings.update(web_port=18086, api_port=18087)
        self.fixture.write_json(self.root / "config/settings.json", self.settings)
        postgres = self.root / "receipts/provision-postgres"
        postgres.mkdir(mode=0o700)
        for name in ("initdb", "postgres"):
            (postgres / name).touch()
        with patch.object(host.os, "umask"), \
                patch.object(host.shutil, "which", side_effect=lambda name: "/usr/bin/" + name), \
                patch.object(host.subprocess, "check_output", return_value=str(postgres)), \
                patch.object(host.subprocess, "run"), \
                patch.object(host.Path, "home", return_value=self.root / "receipts/provision-home"), \
                patch("builtins.print"):
            host.prepare(self.root)
        self.redis_profile = dict(line.split(" ", 1) for line in
                                  (self.root / "config/redis.conf").read_text().splitlines())
        self.assertEqual(self.redis_profile["appenddirname"], "redis-appendonly")
        command = self.redis.command

        def provisioned_response(arguments):
            if arguments[:2] == ["CONFIG", "GET"]:
                self.redis.calls.append(tuple(arguments))
                return [item for key in arguments[2:] for item in (key, self.redis_profile[key])]
            return command(arguments)

        replacement = patch.object(self.redis, "command", side_effect=provisioned_response)
        replacement.start()
        self.addCleanup(replacement.stop)

    def test_provisioned_redis_profile_produces_an_admissible_capture(self):
        before = copy.deepcopy(self.redis.values)
        result = self.invoke_capture()
        descriptor = json.loads(self.descriptor_path.read_bytes())
        self.assertEqual(self.backups_started, 1)
        self.assertTrue((result["backup"] / "COMPLETE").is_file())
        admitted = restore_admission.admit(
            self.root, result["backup"], self.descriptor_path,
            expected_descriptor_sha256=result["descriptor_sha256"],
            controller_directory=self.controller, observed=self.expected_facts())
        self.assertEqual(admitted["audit_predecessor"], descriptor["audit_predecessor"])
        self.assertEqual(self.redis.values, before)
        self.assertFalse(any(call[:2] == ("CONFIG", "SET") for call in self.redis.calls))

    def assert_rejected_profile(self, field, value):
        self.redis_profile[field] = value
        before = self.snapshot()
        self.rejected_capture()
        self.assertEqual(self.backups_started, 0)
        self.assertFalse(self.descriptor_path.exists())
        after = self.snapshot()
        before.pop("deploy.lock", None)
        after.pop("deploy.lock", None)
        self.assertEqual(after, before)

    def test_default_aof_directory_rejects_before_dump(self):
        self.assert_rejected_profile("appenddirname", "appendonlydir")

    def test_other_aof_directory_rejects_before_dump(self):
        self.assert_rejected_profile("appenddirname", "other-appendonly")

    def test_parent_relative_aof_directory_rejects_before_dump(self):
        self.assert_rejected_profile("appenddirname", "../redis-appendonly")

    def test_other_snapshot_filename_rejects_before_dump(self):
        self.assert_rejected_profile("dbfilename", "other.rdb")

    def test_other_data_directory_rejects_before_dump(self):
        self.assert_rejected_profile("dir", str(self.root / "data/other"))


if __name__ == "__main__":
    unittest.main()
