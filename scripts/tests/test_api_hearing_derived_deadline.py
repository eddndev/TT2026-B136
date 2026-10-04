"""Test acceptance assertions with synthetic transport-only evidence."""
import copy
import importlib.util
from pathlib import Path
import unittest


def fixture(blocked=False):
    actor = {'id': 'actor', 'email': 'owner@example.test', 'role': 'owner'}
    values = {'event_time': {'precision': 'date', 'date': '2026-01-06', 'offset': '-06:00'},
              'agreements': []}
    result_command = {'operation_id': 'result-op', 'hearing_id': 'hearing',
                      'result_id': 'result', 'change': {'action': 'record',
                          'expected_revision': 0, 'values': values}}
    source = {'family': 'hearing_result', 'hearing_id': 'hearing',
              'result_id': 'result', 'revision': 1, 'agreement_id': None}
    definition = {'title': 'Synthetic deadline', 'input': {
        'selection': {'case_id': 'case', 'source': {'kind': 'known', 'value': source}}}}
    policies = {'profile': 'fixed', 'source': 'follow', 'calendar': 'undetermined'}
    deadline_command = {'operation_id': 'deadline-op', 'deadline_id': 'deadline',
                        'change': {'action': 'register', 'expected_revision': 0,
                                   'definition': definition, 'tracking': policies}}
    calculation = {'due_at': None if blocked else {'unix_seconds': 1768000000,
                    'nanosecond': 0, 'offset_seconds': 0},
                   'blocks': [{'kind': 'trigger'}] if blocked else []}
    result_draft = {'case_id': 'case', 'actor_id': 'actor', 'command': result_command,
                    'result_revision': 1, 'values': values, 'values_digest': '11' * 32,
                    'submission_digest': '22' * 32, 'anchor': {'revision': 1},
                    'continuation': None, 'attendees': [], 'support': None,
                    'observed_administration': {'revision': 1, 'values_digest': '33' * 32}}
    command = {'case_id': 'case', 'result': result_command, 'deadline': deadline_command}
    responsible = {'id': 'actor', 'email': actor['email']}
    ready = {'state': 'ready', 'command': command, 'result': result_draft,
             'deadline': {'definition': definition, 'tracking': policies,
                          'responsible': responsible, 'result': calculation},
             'review_digest': '44' * 32}
    result = {'id': 'result', 'case_id': 'case', 'hearing_id': 'hearing',
              'revision': 1, 'status': 'recorded', 'reason': None, 'values': values,
              'values_digest': result_draft['values_digest'], 'anchor': result_draft['anchor'],
              'continuation': None, 'attendees': [], 'support': None,
              'recorded_administration_revision': 1, 'recorded_administration_digest': '33' * 32,
              'recorded_by': {key: actor[key] for key in ['id', 'email']},
              'recorded_at': '2026-01-06T08:30:07.123456789-06:00',
              'receipt': {'operation_id': 'result-op', 'action': 'record',
                          'expected_revision': 0, 'submission_digest': '22' * 32}}
    observation = {'role': 'source', 'family': 'hearing_result', 'id': 'result',
                   'revision': 1, 'case_id': 'case', 'hearing_id': 'hearing',
                   'submission_digest': '22' * 32}
    deadline = {'id': 'deadline', 'case_id': 'case', 'revision': 1, 'status': 'active',
                'definition': definition, 'responsible': responsible,
                'recorded_by': {'kind': 'user', **result['recorded_by']},
                'recorded_at': {'unix_seconds': 1767709807, 'nanosecond': 123456789,
                                'offset_seconds': -21600},
                'receipt': {'operation_id': 'deadline-op', 'action': 'register',
                            'expected_revision': 0, 'version': {'kind': 'v2'},
                            'submission_digest': '55' * 32, 'review_digest': '66' * 32,
                            'capture_digest': '77' * 32},
                'tracking': {'policies': policies, 'observations': {'entries': [observation]}},
                'calculation': {'result': calculation, 'material': {'source': {
                    'case_id': 'case', 'reference': source, 'values_digest': '11' * 32,
                    'submission_digest': '22' * 32}}}}
    event = {'sequence': '9007199254740993', 'family': 'hearing_result', 'source_id': 'result',
             'revision': 1, 'case_id': 'case', 'hearing_id': 'hearing', 'operation_id': 'result-op'}
    origin = {'case_id': 'case', 'hearing_id': 'hearing', 'result_id': 'result',
              'result_revision': 1, 'result_operation_id': 'result-op', 'deadline_id': 'deadline',
              'deadline_revision': 1, 'deadline_operation_id': 'deadline-op', 'recorded_by': actor,
              'review_digest': ready['review_digest'], 'capture_digest': '88' * 32,
              'source_event': event}
    record = {'case_id': 'case', 'command': command, 'result': result, 'deadline': deadline,
              'origin': origin, 'review_digest': ready['review_digest'], 'capture_digest': '88' * 32}
    return copy.deepcopy(ready), copy.deepcopy(record), actor


class HearingDerivedDeadlineAcceptance(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        source = Path(__file__).resolve().parents[1] / 'api_hearing_derived_deadline_contract.py'
        spec = importlib.util.spec_from_file_location('compound_acceptance', source)
        cls.helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.helper)

    def test_calculable_and_blocked_capture_preserve_exact_review(self):
        for blocked in [False, True]:
            with self.subTest(blocked=blocked):
                ready, record, actor = fixture(blocked)
                self.helper.assert_creation(ready, record, actor)

    def test_capture_rejects_changed_r1_operation_author_origin_or_review(self):
        changes = [
            (['result', 'id'], 'other'), (['result', 'revision'], 2),
            (['result', 'receipt', 'operation_id'], 'other'),
            (['result', 'recorded_by', 'email'], 'other@example.test'),
            (['deadline', 'receipt', 'operation_id'], 'other'),
            (['deadline', 'calculation', 'result', 'due_at'], None),
            (['deadline', 'tracking', 'policies', 'source'], 'fixed'),
            (['origin', 'deadline_operation_id'], 'other'),
            (['origin', 'recorded_by', 'role'], 'litigator'),
            (['origin', 'capture_digest'], '99' * 32),
            (['review_digest'], '99' * 32),
            (['command', 'deadline', 'change', 'definition', 'title'], 'Changed'),
        ]
        for path, value in changes:
            with self.subTest(path=path), self.assertRaises(AssertionError):
                ready, record, actor = fixture()
                target = record
                for key in path[:-1]:
                    target = target[key]
                target[path[-1]] = value
                self.helper.assert_creation(ready, record, actor)

    def test_source_event_is_exact_bounded_decimal_string(self):
        for field, value in [('sequence', 1), ('sequence', '0'), ('sequence', '01'),
                             ('sequence', str(2 ** 63)), ('family', 'resolution'),
                             ('revision', 2), ('case_id', 'other'), ('hearing_id', 'other'),
                             ('source_id', 'other'), ('operation_id', 'other')]:
            with self.subTest(field=field, value=value), self.assertRaises(AssertionError):
                ready, record, actor = fixture()
                record['origin']['source_event'][field] = value
                self.helper.assert_creation(ready, record, actor)

    def test_shared_capture_requires_exact_nanoseconds_and_original_offset(self):
        for field, value in [('unix_seconds', 1767709808), ('nanosecond', 123456788),
                             ('offset_seconds', 0)]:
            with self.subTest(field=field), self.assertRaises(AssertionError):
                ready, record, actor = fixture()
                record['deadline']['recorded_at'][field] = value
                self.helper.assert_creation(ready, record, actor)

    def test_calculation_source_and_tracking_observation_match_real_hres(self):
        for path in [('calculation', 'material', 'source'),
                     ('tracking', 'observations', 'entries')]:
            with self.subTest(path=path), self.assertRaises(AssertionError):
                ready, record, actor = fixture()
                row = record['deadline']
                for key in path:
                    row = row[key]
                if isinstance(row, list):
                    row = row[0]
                row['submission_digest'] = '99' * 32
                self.helper.assert_creation(ready, record, actor)


if __name__ == '__main__':
    unittest.main()
