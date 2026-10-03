"""Manager-reference lifetime doubles for exact quiesce exit evidence."""
from contextlib import contextmanager
import json
from unittest.mock import patch

from deploy_restore_quiesce_support import UNITS, QuiesceFixture


class HeldReferences:
    manager = ":1.55"

    def __init__(self, fixture):
        self.fixture = fixture

    def check(self, *, timeout):
        fixture = self.fixture
        fixture.assert_locked()
        fixture.assertTrue(fixture.references_active)
        fixture.assertTrue(0 < timeout <= fixture.timeout)
        fixture.reference_events.append(("check",))
        if fixture.owner_changed:
            raise RuntimeError(fixture.secret)

    def status(self, unit, *, timeout):
        self.check(timeout=timeout)
        fixture = self.fixture
        fixture.assertIn(unit, UNITS)
        row = fixture.units[unit]
        if row["MainPID"] != "0":
            fixture.observed_pids[unit] = int(row["MainPID"])
        pid = fixture.observed_pids[unit]
        result = {"pid": pid, "code": int(row["ExecMainCode"]), "status": int(row["ExecMainStatus"]),
                  "started_usec": pid * 1000, "exited_usec": pid * 1000 + 500 if row["MainPID"] == "0" else 0}
        if fixture.metadata_lost and row["MainPID"] == "0":
            result.update(pid=0, code=0, status=0, started_usec=0, exited_usec=0)
        if fixture.status_override:
            fixture.status_override(unit, result)
        fixture.reference_events.append(("status", unit, result["code"]))
        return result


class ReferenceFixture(QuiesceFixture):
    def setUp(self):
        super().setUp()
        self.reference_events = []
        self.references_active = False
        self.refusal_at = None
        self.owner_changed = False
        self.metadata_lost = False
        self.status_override = None
        self.fail_stopped_sync = False
        self.observed_pids = {unit: int(row["MainPID"]) for unit, row in self.units.items()}

    @contextmanager
    def retain_units(self, environment, units, *, timeout):
        self.assert_locked()
        self.assertEqual(environment, {**self.environment, "LC_ALL": "C"})
        self.assertEqual(tuple(units), UNITS)
        self.assertTrue(0 < timeout <= self.timeout)
        self.reference_events.append(("open",))
        acquired = []
        try:
            for unit in units:
                if unit == self.refusal_at:
                    raise RuntimeError(self.secret)
                acquired.append(unit)
                self.reference_events.append(("ref", unit))
            self.references_active = True
            yield HeldReferences(self)
        finally:
            self.assert_locked()
            self.references_active = False
            for unit in reversed(acquired):
                self.reference_events.append(("unref", unit))
            self.reference_events.append(("close",))

    def execute(self, arguments, **kwargs):
        if str(arguments[2]) == "stop":
            self.assertTrue(self.references_active, "stop began without all four live manager references")
            captured = json.loads(self.record.read_bytes())["units"]
            self.assertEqual(set(captured), set(UNITS))
            for unit in (str(value) for value in arguments[3:] if str(value) in UNITS):
                pid = int(self.units[unit]["MainPID"])
                if pid:
                    self.assertEqual(captured[unit]["process"]["pid"], pid)
                    self.assertEqual(captured[unit]["process"]["start_ticks"], 12345)
                    self.assertEqual(captured[unit]["started_usec"], pid * 1000)
                    self.assertIsNone(captured[unit]["exit"])
            self.reference_events.append(("stop-held",))
        return super().execute(arguments, **kwargs)

    def sync(self, directory):
        super().sync(directory)
        if self.record.exists() and json.loads(self.record.read_bytes())["state"] == "stopped":
            self.assertTrue(self.references_active, "references closed before stopped evidence became durable")
            self.reference_events.append(("stopped-sync-held",))
            if self.fail_stopped_sync:
                raise OSError(self.secret)

    @contextmanager
    def controller(self, timeout=240):
        with super().controller(timeout=timeout) as module:
            with patch.object(module, "retain_units", side_effect=self.retain_units, create=True):
                yield module

    def assert_released(self, count=4):
        self.assertFalse(self.references_active)
        self.assertEqual([event[1] for event in self.reference_events if event[0] == "ref"], list(UNITS[:count]))
        self.assertEqual([event[1] for event in self.reference_events if event[0] == "unref"], list(reversed(UNITS[:count])))
        self.assertEqual(self.reference_events[-1], ("close",))
