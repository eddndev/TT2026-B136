"""Check native precautionary observations against their HTTP contracts."""
import copy
import importlib.util
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import Mock, call, patch
from uuid import UUID
from urllib.parse import parse_qs, urlsplit


OPERATION_ID = "00000000-0000-4000-8000-000000000001"
ALERT_ID = "00000000-0000-4000-8000-000000000002"
OTHER_ID = "00000000-0000-4000-8000-000000000003"
RECIPIENT_ID = "00000000-0000-4000-8000-000000000004"


def instant(seconds):
    return {"unix_seconds": seconds, "nanosecond": 0, "offset_seconds": 0}


def alert():
    return {
        "id": ALERT_ID, "recipient_id": RECIPIENT_ID, "occurrence_id": OTHER_ID,
        "subject": {"kind": "precautionary_hearing", "case_id": OPERATION_ID, "id": OTHER_ID},
        "subject_title": "Revision de medidas cautelares", "case_title": "Case",
        "case_reference": "REF", "kind": {
            "kind": "upcoming", "lead_hours": 48, "activity_at": instant(1767312000)},
        "origin": {"revision": 2, "evidence_digest": "ab" * 32},
        "trigger_at": instant(1767139200), "created_at": instant(1767225600),
        "read_at": instant(1767225700), "state": {"kind": "active"},
        "email": {"kind": "disabled"},
    }


class PrecautionaryAlertObservation(unittest.TestCase):
    def setUp(self):
        observation = types.ModuleType("api_resource_hearings_observation")
        observation.inbox = Mock(side_effect=AssertionError("unexpected inbox read"))
        fixtures = types.ModuleType("api_precautionary_fixtures")
        fixtures.TOKEN = "fixture"
        for name in ["actor_token", "collection", "payload", "request", "review", "route"]:
            setattr(fixtures, name, Mock(side_effect=AssertionError("unexpected fixture call")))
        source = Path(__file__).resolve().parents[1] / "api_precautionary_observation.py"
        spec = importlib.util.spec_from_file_location("precautionary_observation_test", source)
        self.helper = importlib.util.module_from_spec(spec)
        with patch.dict(sys.modules, {
            "api_resource_hearings_observation": observation,
            "api_precautionary_fixtures": fixtures,
        }):
            spec.loader.exec_module(self.helper)
        self.operation = {"capture": "unused by stubbed upcoming"}
        self.account = {"id": RECIPIENT_ID, "token": "reader"}
        self.receipt = {
            "operation_id": OPERATION_ID, "checked_at": instant(1767225700), "alert": alert(),
        }

    def observe(self, repeated):
        unread = alert()
        unread["read_at"] = None
        with patch.object(self.helper, "upcoming", return_value=unread) as upcoming, \
                patch.object(self.helper, "uuid4", return_value=UUID(OPERATION_ID)), \
                patch.object(self.helper, "request", side_effect=[self.receipt, repeated]) as request:
            value = self.helper.mark_read(self.operation, self.account)
        upcoming.assert_called_once_with(self.operation, self.account)
        expected = call("POST", "/api/v1/alerts/" + ALERT_ID + "/read",
                        {"operation_id": OPERATION_ID}, token="reader")
        self.assertEqual(request.call_args_list, [expected, expected])
        return value

    def test_repeated_read_accepts_fresh_checked_at_and_preserves_original_receipt(self):
        repeated = copy.deepcopy(self.receipt)
        repeated["checked_at"] = instant(1767225701)
        value = self.observe(repeated)
        self.assertEqual(value, {
            "command": {"operation_id": OPERATION_ID}, "alert": self.receipt["alert"],
        })
        self.assertEqual(value["alert"]["read_at"], instant(1767225700))

    def test_repeated_read_rejects_changed_operation_id(self):
        repeated = copy.deepcopy(self.receipt)
        repeated["operation_id"] = OTHER_ID
        with self.assertRaises(AssertionError):
            self.observe(repeated)

    def test_repeated_read_rejects_changed_alert_or_read_at(self):
        for field, replacement in [("id", OTHER_ID), ("read_at", instant(1767225701)),
                                   ("origin", {"revision": 3, "evidence_digest": "cd" * 32})]:
            repeated = copy.deepcopy(self.receipt)
            repeated["alert"][field] = replacement
            with self.subTest(field=field), self.assertRaises(AssertionError):
                self.observe(repeated)


    def test_agenda_uses_utc_z_boundaries_and_checks_both_status_filters(self):
        hearings, rows = [], []
        for identifier, status, revision, scheduled_at, seconds in [
            (ALERT_ID, "scheduled", 2, "2025-12-31T18:00:00-06:00", 1767225600),
            (OTHER_ID, "cancelled", 3, "2026-01-01T03:30:00+02:00", 1767231000),
        ]:
            values = {"purpose": "review", "scheduled_at": scheduled_at,
                      "modality": "videoconference", "participants": []}
            hearings.append({"capture": {
                "capture_digest": "ab" * 32, "review": {
                    "case_id": OPERATION_ID, "command": {"hearing_id": identifier},
                    "result_revision": revision, "status": status, "resolved_values": values,
                },
            }})
            rows.append({"kind": "precautionary_hearing", "at": instant(seconds),
                         "case_title": "Case", "case_reference": "REF", "case_status": "active",
                         "precautionary_hearing": {
                             "case_id": OPERATION_ID, "id": identifier, "revision": revision,
                             "purpose": "review", "scheduled_at": scheduled_at,
                             "modality": "videoconference", "status": status,
                             "participant_count": 0, "capture_digest": "ab" * 32,
                         }})
        statuses = []

        def request(method, path, *, token):
            self.assertEqual((method, token), ("GET", "reader"))
            endpoint = urlsplit(path)
            self.assertEqual(endpoint.path, "/api/v1/agenda")
            params = parse_qs(endpoint.query, strict_parsing=True)
            self.assertEqual(set(params), {"from", "until", "kind", "hearing_status", "limit"})
            for field in ["from", "until"]:
                self.assertEqual(len(params[field]), 1)
                self.assertRegex(params[field][0], r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$")
            self.assertEqual(params["from"], ["2025-12-31T23:00:00Z"])
            self.assertEqual(params["until"], ["2026-01-01T02:30:00Z"])
            self.assertEqual(params["kind"], ["precautionary_hearing"])
            self.assertEqual(params["limit"], ["100"])
            self.assertEqual(len(params["hearing_status"]), 1)
            status = params["hearing_status"][0]
            self.assertIn(status, ["all", "scheduled", "cancelled"])
            statuses.append(status)
            selected = [row for row in rows if status == "all"
                        or row["precautionary_hearing"]["status"] == status]
            return {
                **{key: params[key][0] for key in ["from", "until", "kind", "hearing_status"]},
                "checked_at": instant(1767225600), "items": copy.deepcopy(selected),
                "complete": True, "next_cursor": None,
            }

        with patch.object(self.helper, "request", side_effect=request) as mocked:
            actual = self.helper.agenda({"case_id": OPERATION_ID}, hearings, token="reader")
        self.assertEqual(actual, rows)
        self.assertEqual(mocked.call_count, 3)
        self.assertEqual(statuses, ["all", "scheduled", "cancelled"])


if __name__ == "__main__":
    unittest.main()
