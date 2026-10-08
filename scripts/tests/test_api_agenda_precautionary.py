"""Keep restored precautionary hearings compatible with combined agenda checks."""
import copy
import importlib.util
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import Mock, patch

from test_api_agenda_resource_hearings import (
    CASE_ID, ITEM_ID, START, deadline, hearing, instant, resource_hearing,
)


def precautionary_hearing(status="scheduled", revision=2):
    return {
        "kind": "precautionary_hearing", "at": instant(START),
        "case_title": "Case", "case_reference": "REF", "case_status": "active",
        "precautionary_hearing": {
            "case_id": CASE_ID, "id": ITEM_ID, "revision": revision,
            "purpose": "review", "scheduled_at": "2025-12-31T18:00:00-06:00",
            "modality": "videoconference", "status": status,
            "participant_count": 1, "capture_digest": "cd" * 32,
        },
    }


class AgendaPrecautionaryHearings(unittest.TestCase):
    def setUp(self):
        support = types.ModuleType("api_deadlines_support")
        support.STATE = Path("unused-deadline-state.json")
        support.TOKEN = "fixture"
        support.request = Mock(side_effect=AssertionError("unexpected HTTP request"))
        source = Path(__file__).resolve().parents[1] / "api-agenda-demo.py"
        spec = importlib.util.spec_from_file_location("agenda_precautionary_test", source)
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

    def test_equal_uuid_and_instant_keep_four_distinct_family_ranks(self):
        rows = [hearing(), deadline(), resource_hearing(), precautionary_hearing()]
        self.assertEqual([self.helper.key(row)[2] for row in rows], [0, 1, 2, 3])
        self.assertEqual(len({self.helper.identity(row) for row in rows}), 4)
        self.assertEqual(sorted(rows, key=self.helper.key), rows)
        self.page(rows)

    def test_page_preserves_current_revision_status_digest_and_original_offset(self):
        for status, revision in [("scheduled", 2), ("cancelled", 3)]:
            with self.subTest(status=status):
                row = precautionary_hearing(status, revision)
                value = self.page([row], kind="precautionary_hearing", hearing_status=status)
                overview = value["items"][0]["precautionary_hearing"]
                self.assertEqual(overview, row["precautionary_hearing"])
                self.assertEqual(overview["revision"], revision)
                self.assertEqual(overview["scheduled_at"], "2025-12-31T18:00:00-06:00")
                self.assertNotIn("deadline", value["items"][0])

    def test_selected_applies_precautionary_status_in_mixed_and_specific_queries(self):
        for status in ["scheduled", "cancelled"]:
            row = precautionary_hearing(status)
            opposite = "cancelled" if status == "scheduled" else "scheduled"
            for kind in ["all", "precautionary_hearing"]:
                with self.subTest(kind=kind, status=status):
                    self.assertTrue(self.helper.selected(row, kind, "all"))
                    self.assertTrue(self.helper.selected(row, kind, status))
                    self.assertFalse(self.helper.selected(row, kind, opposite))
            for kind in ["hearing", "deadline", "resource_hearing"]:
                with self.subTest(kind=kind, status=status):
                    self.assertFalse(self.helper.selected(row, kind, "all"))


if __name__ == "__main__":
    unittest.main()
