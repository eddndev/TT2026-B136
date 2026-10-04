"""Installer reentry rejects foreign publication and confirms marker removal."""
from copy import deepcopy
import json
import os
from unittest.mock import patch

from deploy_controller_installation_support import InstallationFixture, UNITS
from deploy_controller_publication_support import encoded, identity, put, tree


class ControllerInstallationReconciliationTests(InstallationFixture):
    def test_prepared_reentry_rejects_foreign_publication_before_stopping(self):
        self.refusal_at = UNITS[0]
        with self.controller() as product, self.assertRaises(RuntimeError):
            self.invoke(product)
        self.assertEqual(json.loads(self.intent_record.read_bytes())["state"], "prepared")
        self.assertIn(("open",), self.reference_events)
        self.assertFalse(self.references_active)
        self.assertEqual(self.stops(), [])
        self.assertFalse(any(event[0] == "mask" for event in self.events))
        self.refusal_at = None
        put(self.publication / "journal.json", encoded({"format": "unrelated-publication"}))
        before = tree(self.root), tree(self.home)
        units = deepcopy(self.units)
        with self.controller() as product, self.assertRaises(RuntimeError) as error:
            self.invoke(product)
        self.assertNotIn(self.secret, str(error.exception))
        self.assertEqual(self.stops(), [], "foreign publication reached a service stop")
        self.assertFalse(any(event[0] in ("mask", "publish", "approve", "start") for event in self.events))
        self.assertEqual(self.units, units)
        self.assertEqual((tree(self.root), tree(self.home)), before)
        self.assertEqual(self.preserved(), self.protected)

    def test_missing_active_marker_reentry_flushes_its_parent_after_lost_sync(self):
        with self.controller() as product:
            closed = self.invoke(product)
        self.authorize(closed)
        marker = self.operation.parent / "active.json"
        self.assertTrue(marker.exists())
        parent_identity = identity(marker.parent)
        failed = False
        with self.controller() as product:
            previous_sync = os.fsync

            def fail_after_unlink(descriptor):
                nonlocal failed
                info = os.fstat(descriptor)
                if ((info.st_dev, info.st_ino) == parent_identity
                        and not marker.exists() and not failed):
                    failed = True
                    raise OSError(self.secret)
                previous_sync(descriptor)

            with patch.object(os, "fsync", side_effect=fail_after_unlink), self.assertRaises(RuntimeError) as error:
                self.invoke(product, reopen=True)
        self.assertTrue(failed, "fault did not reach the removed marker's parent")
        self.assertNotIn(self.secret, str(error.exception))
        self.assertFalse(marker.exists())
        self.assertEqual(json.loads(self.intent_record.read_bytes())["state"], "installed")
        before = tree(self.root), tree(self.home)
        starts = [event for event in self.events if event[0] == "start"]
        stops = self.stops()
        flushed = []
        with self.controller() as product:
            previous_sync = os.fsync

            def observe_sync(descriptor):
                info = os.fstat(descriptor)
                previous_sync(descriptor)
                if (info.st_dev, info.st_ino) == parent_identity:
                    self.assertFalse(marker.exists())
                    flushed.append(parent_identity)

            with patch.object(os, "fsync", side_effect=observe_sync):
                receipt = self.invoke(product, reopen=True)
        self.assertEqual(receipt, {**closed, "state": "installed"})
        self.assertTrue(flushed, "installed receipt preceded durable confirmation of marker removal")
        self.assertEqual([event for event in self.events if event[0] == "start"], starts)
        self.assertEqual(self.stops(), stops)
        self.assertEqual((tree(self.root), tree(self.home)), before)
        self.assertEqual(self.preserved(), self.protected)
