"""Publication rechecks identities and reconciles uncertain durable exchanges."""
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch

from deploy_controller_publication_support import (
    Fixture, OTHER_OPERATION, REJECTIONS, identity, inventory, journal, module, put, tree,
)


class ControllerPublicationReentryTests(unittest.TestCase):
    def setUp(self):
        self.publisher = module()

    def test_prepared_journal_does_not_authorize_changed_directory_or_staged_bytes(self):
        for attack in ("tools_inode", "candidate_inode", "candidate_bytes", "operation_inode"):
            with self.subTest(attack=attack):
                fixture = Fixture(self)
                real_replace = os.replace
                replaced = False
                foreign = None

                def replace(source, destination):
                    nonlocal replaced, foreign
                    result = real_replace(source, destination)
                    if Path(destination) != fixture.record or replaced:
                        return result
                    if json.loads(fixture.record.read_bytes())["state"] != "prepared":
                        return result
                    replaced = True
                    with self.assertRaises(BlockingIOError), journal.locked(fixture.root):
                        pass
                    if attack in ("tools_inode", "candidate_inode"):
                        target = fixture.tools if attack == "tools_inode" else fixture.exchange
                        moved = fixture.base / "retained-original"
                        target.rename(moved)
                        target.mkdir(mode=0o700)
                        for path in moved.iterdir():
                            put(target / path.name, path.read_bytes())
                        foreign = target, tree(target)
                    elif attack == "candidate_bytes":
                        put(fixture.exchange / "alpha.py", b"VALUE = 'modified after staging'\n")
                        foreign = fixture.exchange, tree(fixture.exchange)
                    else:
                        fixture.operation.rename(fixture.base / "retained-operation")
                        fixture.operation.mkdir(mode=0o700)
                        put(fixture.operation / "foreign.txt", b"another inode owns this directory\n")
                        foreign = fixture.operation, tree(fixture.operation)
                    return result

                with patch.object(os, "replace", side_effect=replace):
                    with patch.object(self.publisher, "exchange_directories",
                                      side_effect=AssertionError("changed identity reached exchange")):
                        with self.assertRaises(REJECTIONS):
                            fixture.publish(self.publisher)
                self.assertTrue(replaced, "fixture did not reach durable preparation")
                self.assertEqual(tree(foreign[0]), foreign[1], "cleanup removed a replacement inode")
                self.assertEqual(inventory(fixture.tools), fixture.old)
                fixture.assert_preserved(self)

    def test_uncertain_exchange_and_each_parent_flush_reconcile_without_second_exchange(self):
        for failure in ("exchange", "root_sync", "operation_sync", "receipt_sync"):
            with self.subTest(failure=failure):
                fixture = Fixture(self)
                real_exchange = self.publisher.exchange_directories
                real_flush, real_replace = os.fsync, os.replace
                exchanged = False
                published = False
                failed = False

                def exchange(left, right):
                    nonlocal exchanged, failed
                    real_exchange(left, right)
                    exchanged = True
                    if failure == "exchange":
                        failed = True
                        raise OSError("fixture lost confirmation after directory exchange")

                def replace(source, destination):
                    nonlocal published
                    result = real_replace(source, destination)
                    if Path(destination) == fixture.record:
                        published = json.loads(fixture.record.read_bytes())["state"] == "published"
                    return result

                def flush(descriptor):
                    nonlocal failed
                    row = os.fstat(descriptor)
                    current = row.st_dev, row.st_ino
                    expected = (identity(fixture.root) if failure == "root_sync"
                                else identity(fixture.operation) if fixture.operation.exists() else None)
                    matching = failure in ("root_sync", "operation_sync", "receipt_sync")
                    matching = matching and current == expected
                    if failure == "receipt_sync":
                        matching = matching and published
                    elif failure == "operation_sync":
                        matching = matching and not published
                    if exchanged and matching and not failed:
                        failed = True
                        raise OSError("fixture interrupted publication directory durability")
                    return real_flush(descriptor)

                with patch.object(self.publisher, "exchange_directories", side_effect=exchange):
                    with patch.object(os, "replace", side_effect=replace):
                        with patch.object(os, "fsync", side_effect=flush):
                            with self.assertRaises(OSError):
                                fixture.publish(self.publisher)
                self.assertTrue(failed, "fixture did not reach the selected uncertainty boundary")
                self.assertEqual(inventory(fixture.tools), fixture.new)
                self.assertEqual(tree(fixture.exchange), fixture.old_tree)
                self.assertTrue(fixture.record.is_file())
                before = tree(fixture.operation)
                for overrides in ({"operation_id": OTHER_OPERATION},
                                  {"expected_manifest_sha256": "0" * 64},
                                  {"expected_previous_sha256": "0" * 64}):
                    with self.assertRaises(REJECTIONS):
                        fixture.publish(self.publisher, **overrides)
                    self.assertEqual(tree(fixture.operation), before)
                    self.assertFalse((fixture.operation.parent / OTHER_OPERATION).exists())
                flushed = []

                def resumed_flush(descriptor):
                    row = os.fstat(descriptor)
                    flushed.append((row.st_dev, row.st_ino))
                    return real_flush(descriptor)

                with patch.object(self.publisher, "exchange_directories",
                                  side_effect=AssertionError("reentry exchanged already installed sources")):
                    with patch.object(os, "fsync", side_effect=resumed_flush):
                        receipt = fixture.publish(self.publisher)
                for parent in (fixture.root, fixture.operation):
                    self.assertIn(identity(parent), flushed)
                fixture.assert_published(self, receipt)

    def test_published_reentry_reobserves_both_sides_and_preserves_unowned_replacements(self):
        for attack in ("none", "installed_bytes", "retained_bytes", "operation_inode"):
            with self.subTest(attack=attack):
                fixture = Fixture(self)
                receipt = fixture.publish(self.publisher)
                fixture.assert_published(self, receipt)
                if attack == "installed_bytes":
                    put(fixture.tools / "alpha.py", b"VALUE = 'unapproved installed source'\n")
                elif attack == "retained_bytes":
                    put(fixture.exchange / "alpha.py", b"VALUE = 'unapproved retained source'\n")
                elif attack == "operation_inode":
                    retained = fixture.base / "retained-operation"
                    fixture.operation.rename(retained)
                    fixture.operation.mkdir(mode=0o700)
                    put(fixture.record, (retained / "journal.json").read_bytes())
                    (retained / "exchange").rename(fixture.exchange)
                    put(fixture.operation / "foreign.txt", b"replacement directory must survive\n")
                before = tree(fixture.tools), tree(fixture.operation)
                with patch.object(self.publisher, "exchange_directories",
                                  side_effect=AssertionError("published reentry performed exchange")):
                    if attack == "none":
                        self.assertEqual(fixture.publish(self.publisher), receipt)
                    else:
                        with self.assertRaises(REJECTIONS):
                            fixture.publish(self.publisher)
                self.assertEqual((tree(fixture.tools), tree(fixture.operation)), before)
                fixture.assert_preserved(self)


if __name__ == "__main__":
    unittest.main()
