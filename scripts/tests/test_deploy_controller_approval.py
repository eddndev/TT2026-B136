"""An exact published generation gains one recoverable local approval."""
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch

from deploy_controller_approval_support import (
    Fixture, OPERATION, REJECTIONS, approver, digest, encoded, identity, journal, put, tree,
)


class ControllerApprovalTests(unittest.TestCase):
    def setUp(self):
        self.product = approver()

    def test_approval_records_exact_publication_and_requires_explicit_predecessor_before_writing(self):
        fixture = Fixture(self)
        sources = fixture.sources_snapshot()
        first = fixture.approve(self.product)
        fixture.assert_approved(self, first)
        self.assertEqual(fixture.sources_snapshot(), sources)
        first_path = fixture.operation_approval
        first_tree = tree(first_path)
        current_tree = tree(fixture.current_approval)
        self.assertEqual(fixture.approve(self.product), first)
        self.assertEqual(tree(first_path), first_tree)
        self.assertEqual(tree(fixture.current_approval), current_tree)
        with journal.locked(fixture.root), self.assertRaises(BlockingIOError):
            fixture.approve(self.product)

        fixture.advance()
        before = tree(fixture.operation), tree(fixture.current_approval)
        for predecessor in (None, "0" * 64):
            with self.subTest(predecessor=predecessor), self.assertRaises(REJECTIONS):
                fixture.approve(self.product, predecessor)
            self.assertEqual((tree(fixture.operation), tree(fixture.current_approval)), before)
        second = fixture.approve(self.product, first["approval_sha256"])
        fixture.assert_approved(self, second, first["approval_sha256"])
        self.assertEqual(tree(first_path), first_tree)
        selected = tree(fixture.current_approval)
        with self.assertRaises(REJECTIONS):
            fixture.approve(self.product, operation_id=OPERATION)
        self.assertEqual(tree(fixture.current_approval), selected)

    def test_unpublished_or_replaced_generation_and_foreign_approval_never_gain_authority(self):
        for attack in ("prepared", "journal_hash", "inventory_hash", "generation_inode",
                       "foreign_current", "foreign_operation", "legacy_cache"):
            with self.subTest(attack=attack):
                fixture = Fixture(self)
                overrides = {}
                if attack == "prepared":
                    changed = {**fixture.publication, "state": "prepared"}
                    put(fixture.record, encoded(changed))
                    overrides["expected_publication_sha256"] = digest(fixture.record.read_bytes())
                elif attack == "journal_hash":
                    overrides["expected_publication_sha256"] = "0" * 64
                elif attack == "inventory_hash":
                    overrides["expected_installed_sha256"] = "0" * 64
                elif attack == "generation_inode":
                    retained = fixture.base / "same-bytes-original"
                    fixture.tools.rename(retained)
                    fixture.tools.mkdir(mode=0o700)
                    for path in retained.iterdir():
                        put(fixture.tools / path.name, path.read_bytes())
                elif attack == "foreign_current":
                    put(fixture.current_approval, b"unrelated private selection\n")
                elif attack == "foreign_operation":
                    put(fixture.operation_approval, b"another operation owns these bytes\n")
                else:
                    put(fixture.tools / "__pycache__/preserved.pyc", b"legacy cache remains private")
                before = tree(fixture.operation.parent), fixture.sources_snapshot()
                with self.assertRaises(REJECTIONS):
                    fixture.approve(self.product, **overrides)
                self.assertEqual((tree(fixture.operation.parent), fixture.sources_snapshot()), before)
                fixture.assert_preserved(self)

    def test_uncertain_promotion_reenters_exact_record_and_flushes_before_success(self):
        for boundary in ("before_replace", "after_replace_before_sync"):
            with self.subTest(boundary=boundary):
                fixture = Fixture(self)
                first = fixture.approve(self.product)
                previous = fixture.current_approval.read_bytes()
                fixture.advance()
                sources = fixture.sources_snapshot()
                real_replace, real_sync = os.replace, os.fsync
                failed, promoted = False, False
                flushed = []

                def sync(descriptor):
                    nonlocal failed
                    row = os.fstat(descriptor)
                    observed = row.st_dev, row.st_ino
                    if promoted and observed == identity(fixture.operation.parent) and not failed:
                        failed = True
                        raise OSError("fixture lost approval directory durability")
                    flushed.append(observed)
                    return real_sync(descriptor)

                def replace(source, destination):
                    nonlocal failed, promoted
                    if Path(destination) == fixture.current_approval:
                        self.assertIn(identity(fixture.operation_approval), flushed)
                        self.assertIn(identity(fixture.operation), flushed)
                        with self.assertRaises(BlockingIOError), journal.locked(fixture.root):
                            pass
                        if boundary == "before_replace" and not failed:
                            failed = True
                            raise OSError("fixture lost approval promotion before replace")
                        result = real_replace(source, destination)
                        promoted = True
                        return result
                    return real_replace(source, destination)

                with patch.object(os, "replace", side_effect=replace), patch.object(os, "fsync", side_effect=sync):
                    with self.assertRaises(OSError):
                        fixture.approve(self.product, first["approval_sha256"])
                self.assertTrue(failed, "fixture did not reach the selected uncertainty boundary")
                operation_before = tree(fixture.operation_approval)
                expected = encoded(fixture.expected(first["approval_sha256"]))
                self.assertEqual(fixture.current_approval.read_bytes(), expected if promoted else previous)
                self.assertEqual(fixture.sources_snapshot(), sources)
                flushed.clear()

                def resumed_sync(descriptor):
                    row = os.fstat(descriptor)
                    flushed.append((row.st_dev, row.st_ino))
                    return real_sync(descriptor)

                with patch.object(os, "fsync", side_effect=resumed_sync):
                    receipt = fixture.approve(self.product, first["approval_sha256"])
                fixture.assert_approved(self, receipt, first["approval_sha256"])
                self.assertEqual(tree(fixture.operation_approval), operation_before)
                self.assertIn(identity(fixture.operation.parent), flushed)
                before = tree(fixture.current_approval), tree(fixture.operation_approval)
                with self.assertRaises(REJECTIONS):
                    fixture.approve(self.product, "0" * 64)
                self.assertEqual((tree(fixture.current_approval), tree(fixture.operation_approval)), before)
                self.assertEqual(fixture.sources_snapshot(), sources)


if __name__ == "__main__":
    unittest.main()
