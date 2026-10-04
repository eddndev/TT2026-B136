"""Verify combined agenda acceptance without HTTP or environment dependencies."""
import copy
from datetime import datetime, timezone
import importlib.util
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import Mock, patch


ITEM_ID = "00000000-0000-0000-0000-000000000007"
CASE_ID = "00000000-0000-0000-0000-000000000001"
RESOURCE_ID = "00000000-0000-0000-0000-000000000002"
ASSOCIATION_ID = "00000000-0000-0000-0000-000000000003"
START = 1767225600


def instant(seconds):
    return {"unix_seconds": seconds, "nanosecond": 0, "offset_seconds": 0}


def hearing(status="scheduled"):
    return {"kind": "hearing", "at": instant(START), "hearing": {
        "id": ITEM_ID, "case_id": CASE_ID, "status": status,
        "scheduled_at": "2026-01-01T00:00:00Z"}}


def deadline():
    return {"kind": "deadline", "at": instant(START), "case_title": "Case",
            "case_reference": "REF", "case_status": "active", "deadline": {
                "id": ITEM_ID, "case_id": CASE_ID, "status": "active", "receipt_kind": "v2",
                "review_state": "accepted", "calculation_blocked": False,
                "calculation_due_at": instant(START), "operational": {
                    "freshness": "current", "checked_at": instant(START),
                    "due_at": instant(START)}}}


def resource_hearing(seconds=START):
    scheduled = datetime.fromtimestamp(seconds, timezone.utc).isoformat().replace("+00:00", "Z")
    return {"kind": "resource_hearing", "at": instant(seconds), "case_title": "Case",
            "case_reference": "REF", "case_status": "closed", "resource_hearing": {
                "case_id": CASE_ID, "resource_id": RESOURCE_ID, "id": ITEM_ID,
                "revision": 1, "kind": "appeal_arguments", "scheduled_at": scheduled,
                "modality": "in_person", "participant_count": 0,
                "association_id": ASSOCIATION_ID, "capture_digest": "ab" * 32}}


class AgendaResourceHearings(unittest.TestCase):
    def setUp(self):
        support = types.ModuleType("api_deadlines_support")
        support.STATE = Path("unused-deadline-state.json")
        support.TOKEN = "fixture"
        support.request = Mock(side_effect=AssertionError("unexpected HTTP request"))
        source = Path(__file__).resolve().parents[1] / "api-agenda-demo.py"
        spec = importlib.util.spec_from_file_location("agenda_acceptance_test", source)
        self.helper = importlib.util.module_from_spec(spec)
        with patch.dict(sys.modules, {"api_deadlines_support": support}):
            spec.loader.exec_module(self.helper)
        self.params = {**self.helper.RANGE, "until": "2026-01-02T00:00:00Z", "limit": 20}

    def page(self, items, **filters):
        params = {**self.params, **filters}
        response = {field: params[field] for field in self.helper.RANGE}
        response.update(checked_at=instant(START), items=copy.deepcopy(items),
                        complete=True, next_cursor=None)
        with patch.object(self.helper, "request", return_value=response) as request:
            actual = self.helper.page(params, token="reader")
        request.assert_called_once_with("GET", self.helper.endpoint(params), token="reader")
        self.assertEqual(actual, response)
        return actual

    def test_equal_uuid_and_instant_keep_three_distinct_family_ranks(self):
        rows = [hearing(), deadline(), resource_hearing()]
        self.assertEqual([self.helper.key(row)[2] for row in rows], [0, 1, 2])
        self.assertEqual(len({self.helper.key(row) for row in rows}), 3)
        self.assertEqual(len({self.helper.identity(row) for row in rows}), 3)
        self.assertEqual(sorted(rows, key=self.helper.key), rows)

    def test_resource_hearing_page_uses_its_own_projection_and_original_offset(self):
        row = resource_hearing()
        row["resource_hearing"]["scheduled_at"] = "2025-12-31T18:00:00-06:00"
        value = self.page([row], kind="resource_hearing", hearing_status="scheduled")
        self.assertEqual(value["items"][0]["resource_hearing"]["resource_id"], RESOURCE_ID)
        self.assertNotIn("hearing", value["items"][0])
        self.assertNotIn("status", value["items"][0]["resource_hearing"])

    def test_mixed_page_accepts_colliding_ids_and_all_supported_filters(self):
        for kind, status, rows in [
            ("all", "all", [hearing(), deadline(), resource_hearing()]),
            ("all", "scheduled", [hearing(), deadline(), resource_hearing()]),
            ("all", "cancelled", [hearing("cancelled"), deadline()]),
            ("hearing", "cancelled", [hearing("cancelled")]),
            ("deadline", "scheduled", [deadline()]),
            ("resource_hearing", "all", [resource_hearing()]),
            ("resource_hearing", "scheduled", [resource_hearing()]),
        ]:
            with self.subTest(kind=kind, status=status):
                self.page(rows, kind=kind, hearing_status=status)

    def test_page_rejects_rows_outside_the_requested_family_or_status(self):
        for kind, status, rows in [
            ("hearing", "all", [resource_hearing()]),
            ("deadline", "scheduled", [resource_hearing()]),
            ("resource_hearing", "all", [hearing()]),
            ("all", "cancelled", [resource_hearing()]),
            ("all", "scheduled", [hearing("cancelled")]),
            ("hearing", "cancelled", [hearing()]),
        ]:
            with self.subTest(kind=kind, status=status), self.assertRaises(AssertionError):
                self.page(rows, kind=kind, hearing_status=status)

    def test_page_rejects_dates_outside_the_half_open_interval(self):
        for seconds in [START - 1, START + 86400]:
            with self.subTest(seconds=seconds), self.assertRaises(AssertionError):
                self.page([resource_hearing(seconds)])

    def test_resource_hearing_overview_rejects_invalid_scope_and_invented_fields(self):
        changes = [
            ("case_id", "not-a-case"), ("resource_id", "not-a-resource"),
            ("association_id", "not-an-association"), ("revision", 2),
            ("kind", "intermediate"), ("modality", "unknown"),
            ("participant_count", 33), ("capture_digest", "AB" * 32),
            ("scheduled_at", "2026-01-01T00:00:01Z"),
            ("venue", "Private address"), ("note", "Private note"),
            ("status", "scheduled"), ("recorded_stage", 1),
        ]
        for field, value in changes:
            row = resource_hearing()
            row["resource_hearing"][field] = value
            with self.subTest(field=field), self.assertRaises(AssertionError):
                self.page([row])
        row = resource_hearing()
        del row["resource_hearing"]["resource_id"]
        with self.assertRaises(AssertionError):
            self.page([row])


if __name__ == "__main__":
    unittest.main()
