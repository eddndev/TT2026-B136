"""Exact, private and durable publication of a complete controller source set."""
import errno
import json
import os
from pathlib import Path
import stat
import unittest
from unittest.mock import patch

from deploy_controller_publication_support import (
    Fixture, REJECTIONS, descriptor_inventory, encoded,
    identity, inventory, journal, module, put, tree,
)


class ControllerPublicationTests(unittest.TestCase):
    def setUp(self):
        self.publisher = module()

    def test_exact_inventory_replaces_sources_and_preserves_all_unrelated_private_state(self):
        fixture = Fixture(self)
        candidate = tree(fixture.stage)
        previous_inode = identity(fixture.tools)
        receipt = fixture.publish(self.publisher)
        fixture.assert_published(self, receipt)
        self.assertEqual(identity(fixture.exchange), previous_inode)
        self.assertEqual(tree(fixture.stage), candidate)
        self.assertFalse(fixture.tools.is_symlink())
        self.assertFalse((fixture.tools / "retired.py").exists())
        for path in (fixture.root / "maintenance/controllers", fixture.operation,
                     fixture.tools):
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o700)
        for path in [fixture.record, *fixture.tools.iterdir()]:
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)

    def test_invalid_inventory_paths_bounds_permissions_and_busy_lock_reject_before_journal(self):
        cases = ("missing", "extra", "symlink", "fifo", "directory", "bytecode", "hash",
                 "size", "mode", "manifest_mode", "manifest_duplicate", "manifest_extra",
                 "manifest_path", "empty", "too_many", "too_large", "total", "wrong_manifest", "wrong_previous",
                 "root_symlink", "tools_symlink", "old_extra", "bad_operation", "busy")
        for name in cases:
            with self.subTest(name=name):
                fixture = Fixture(self)
                source = fixture.sources / "alpha.py"
                overrides = {}
                if name == "missing":
                    source.unlink()
                elif name == "extra":
                    put(fixture.sources / "extra.py", b"VALUE = 1\n")
                elif name == "symlink":
                    source.unlink()
                    source.symlink_to(fixture.tools / "alpha.py")
                elif name == "fifo":
                    source.unlink()
                    os.mkfifo(source, 0o600)
                elif name == "directory":
                    (fixture.sources / "nested").mkdir(mode=0o700)
                elif name == "bytecode":
                    put(fixture.sources / "alpha.pyc", b"untrusted bytecode")
                elif name in ("hash", "size"):
                    fixture.manifest["files"]["alpha.py"][
                        "sha256" if name == "hash" else "bytes"] = "0" * 64 if name == "hash" else 1
                    fixture.save_manifest()
                elif name == "mode":
                    source.chmod(0o644)
                elif name == "manifest_mode":
                    (fixture.stage / "manifest.json").chmod(0o644)
                elif name == "manifest_duplicate":
                    raw = encoded(fixture.manifest).replace(b'{"files":', b'{"version":1,"files":', 1)
                    fixture.save_manifest(raw)
                elif name == "manifest_extra":
                    fixture.manifest["authority"] = "caller cannot select configuration"
                    fixture.save_manifest()
                elif name == "manifest_path":
                    fixture.manifest["files"]["../escape.py"] = fixture.new["alpha.py"]
                    fixture.save_manifest()
                elif name == "empty":
                    for path in fixture.sources.iterdir():
                        path.unlink()
                    fixture.manifest["files"] = {}
                    fixture.save_manifest()
                elif name == "too_many":
                    for index in range(65):
                        put(fixture.sources / f"source_{index}.py", b"VALUE = 1\n")
                    fixture.manifest["files"] = inventory(fixture.sources)
                    fixture.save_manifest()
                elif name == "too_large":
                    put(source, b"#" * (256 * 1024 + 1))
                    fixture.manifest["files"] = inventory(fixture.sources)
                    fixture.save_manifest()
                elif name == "total":
                    for index in range(33):
                        put(fixture.sources / f"source_{index}.py", b"#" * (256 * 1024))
                    fixture.manifest["files"] = inventory(fixture.sources)
                    fixture.save_manifest()
                elif name == "wrong_manifest":
                    overrides["expected_manifest_sha256"] = "0" * 64
                elif name == "wrong_previous":
                    overrides["expected_previous_sha256"] = "0" * 64
                elif name == "root_symlink":
                    alias = fixture.base / "alias"
                    alias.symlink_to(fixture.root, target_is_directory=True)
                    overrides["root"] = alias
                elif name == "tools_symlink":
                    destination = fixture.base / "redirected-tools"
                    fixture.tools.rename(destination)
                    fixture.tools.symlink_to(destination, target_is_directory=True)
                elif name == "old_extra":
                    put(fixture.tools / "private-extra.txt", b"must not disappear")
                elif name == "bad_operation":
                    overrides["operation_id"] = "00000000-0000-0000-0000-000000000000"
                before = tree(fixture.tools)
                if name == "busy":
                    with journal.locked(fixture.root), self.assertRaises(BlockingIOError):
                        fixture.publish(self.publisher)
                else:
                    with self.assertRaises(REJECTIONS):
                        fixture.publish(self.publisher, **overrides)
                self.assertEqual(tree(fixture.tools), before)
                self.assertFalse(fixture.operation.exists())
                fixture.assert_preserved(self)

    def test_exchange_keeps_pinned_readers_and_requires_file_and_parent_fsync_without_fallback(self):
        fixture = Fixture(self)
        events = []
        real_sync, real_exchange = os.fsync, self.publisher.exchange_directories
        old_descriptor = os.open(fixture.tools, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        self.addCleanup(os.close, old_descriptor)

        def flush(descriptor):
            row = os.fstat(descriptor)
            events.append(("sync", row.st_dev, row.st_ino))
            return real_sync(descriptor)

        def exchange(left, right):
            self.assertEqual((Path(left), Path(right)), (fixture.tools, fixture.exchange))
            with self.assertRaises(BlockingIOError), journal.locked(fixture.root):
                pass
            record = json.loads(fixture.record.read_bytes())
            self.assertEqual(record["state"], "prepared")
            for path in [fixture.record, fixture.exchange, *fixture.exchange.iterdir()]:
                self.assertIn(("sync", *identity(path)), events)
            self.assertIn(("sync", *identity(fixture.operation)), events)
            self.assertEqual(inventory(fixture.tools), fixture.old)
            self.assertEqual(inventory(fixture.exchange), fixture.new)
            events.append(("exchange",))
            with patch.object(os, "rename", side_effect=AssertionError("sequential rename fallback")):
                with patch.object(os, "replace", side_effect=AssertionError("sequential replace fallback")):
                    real_exchange(left, right)
            self.assertEqual(descriptor_inventory(old_descriptor), fixture.old)
            descriptor = os.open(fixture.tools, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
            try:
                self.assertEqual(descriptor_inventory(descriptor), fixture.new)
            finally:
                os.close(descriptor)

        with patch.object(os, "fsync", side_effect=flush):
            with patch.object(self.publisher, "exchange_directories", side_effect=exchange) as observed:
                receipt = fixture.publish(self.publisher)
        observed.assert_called_once()
        position = events.index(("exchange",))
        for parent in (fixture.root, fixture.operation):
            self.assertIn(("sync", *identity(parent)), events[position + 1:])
        fixture.assert_published(self, receipt)

        for code in (errno.ENOSYS, errno.EXDEV):
            with self.subTest(exchange_error=code):
                unsupported = Fixture(self)
                with patch.object(self.publisher, "exchange_directories",
                                  side_effect=OSError(code, "fixture unsupported exchange")):
                    with self.assertRaises(OSError):
                        unsupported.publish(self.publisher)
                self.assertEqual(tree(unsupported.tools), unsupported.old_tree)
                self.assertEqual(inventory(unsupported.exchange), unsupported.new)
                self.assertEqual(json.loads(unsupported.record.read_bytes())["state"], "prepared")
                unsupported.assert_preserved(self)


if __name__ == "__main__":
    unittest.main()
