"""Activation preserves the previous release and rejects incompatible schemas."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
from release import activate, rollback


class FakeRuntime:
    def __init__(self, root, fail=False):
        self.root, self.fail = root, fail
        self.events = []

    def stop(self):
        self.events.append("stop")

    def backup(self):
        self.events.append("backup")

    def initialize(self, target):
        self.events.append("initialize")

    def start(self, target):
        self.events.append(target.name)
        if self.fail and target.name == "new":
            raise RuntimeError("readiness failed")


class ActivationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "releases").mkdir()
        (self.root / "config").mkdir()
        self.links = {}
        self.addCleanup(patch.stopall)
        patch("release.linked", side_effect=lambda root, name: self.links.get(name)).start()
        patch("release.point", side_effect=lambda root, name, target: self.links.update({name: target})).start()
        self.old = self.make_release("old", "v1.0.0")
        self.new = self.make_release("new", "v1.1.0")
        (self.root / "config/schema").write_text("same")

    def make_release(self, name, version, schema="same"):
        path = self.root / "releases" / name
        path.mkdir()
        (path / "release.json").write_text(json.dumps(
            {"version": version, "schema": schema, "commit": "a" * 40}))
        return path

    def link(self, target):
        self.links["current"] = target

    def test_success_records_previous_after_health(self):
        self.link(self.old)
        runtime = FakeRuntime(self.root)
        activate(self.root, self.new, runtime)
        self.assertEqual(self.links["current"], self.new)
        self.assertEqual(self.links["previous"], self.old)
        self.assertEqual(runtime.events, ["stop", "backup", "initialize", "new"])

    def test_failed_health_restores_previous_and_still_fails(self):
        self.link(self.old)
        runtime = FakeRuntime(self.root, fail=True)
        with self.assertRaisesRegex(RuntimeError, "readiness failed"):
            activate(self.root, self.new, runtime)
        self.assertEqual(self.links["current"], self.old)
        self.assertEqual(runtime.events[-2:], ["stop", "old"])

    def test_incompatible_schema_does_not_stop_or_migrate(self):
        self.link(self.old)
        target = self.make_release("incompatible", "v2.0.0", "changed")
        runtime = FakeRuntime(self.root)
        with self.assertRaisesRegex(ValueError, "schema"):
            activate(self.root, target, runtime)
        self.assertEqual(runtime.events, [])

    def test_failed_backup_preserves_old_release(self):
        self.link(self.old)
        runtime = FakeRuntime(self.root)
        with patch.object(runtime, "backup", side_effect=RuntimeError("backup failed")):
            with self.assertRaisesRegex(RuntimeError, "backup failed"):
                activate(self.root, self.new, runtime)
        self.assertEqual(self.links["current"], self.old)
        self.assertEqual(runtime.events[-1], "old")

    def test_manual_rollback_swaps_release_without_restoring_data(self):
        self.link(self.new)
        self.links["previous"] = self.old
        runtime = FakeRuntime(self.root)
        rollback(self.root, runtime)
        self.assertEqual(self.links["current"], self.old)
        self.assertEqual(self.links["previous"], self.new)

    def test_delayed_older_tag_does_not_downgrade(self):
        self.link(self.new)
        runtime = FakeRuntime(self.root)
        with self.assertRaisesRegex(ValueError, "older"):
            activate(self.root, self.old, runtime)
        self.assertEqual(runtime.events, [])


if __name__ == "__main__":
    unittest.main()
