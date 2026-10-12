#!/usr/bin/env python3
"""Compare eight controlled legal fixtures with independent dates over real HTTP."""
import json
import sys
import time
from copy import deepcopy
from datetime import datetime
from urllib.parse import urlencode
from uuid import uuid4

from api_legal_corpus_fixtures import compound, fixture, source_deadline
from api_deadlines_support import prepare, submit
from api_hearing_derived_deadline_fixtures import (
    WORK, prepare as prepare_compound, request, route, submit as submit_compound,
)

STATE = WORK / 'legal-corpus-api-state.json'
# These dates and hours are derived in the versioned corpus sheets, not by the evaluator.
EXPECTED = {
    'initial': '2026-02-12T18:00:00-06:00',
    'extension': '2026-03-12T18:00:00-06:00',
    'oral': '2026-10-09T10:50:00-06:00',
    'written': '2026-10-16T14:00:00-06:00',
    'control_order': '2026-10-19T15:00:00-06:00',
    'control_sentence': '2026-10-21T15:00:00-06:00',
    'withdrawal': '2026-10-19T15:00:00-06:00',
    'trial_sentence': '2026-10-28T15:00:00-06:00',
    'written_holiday': '2026-11-03T15:00:00-06:00',
}


def save(saved):
    STATE.write_text(json.dumps(saved, sort_keys=True, ensure_ascii=True), encoding='ascii')
    STATE.chmod(0o600)


def instant(expected):
    return {'unix_seconds': int(datetime.fromisoformat(expected).timestamp()),
            'nanosecond': 0, 'offset_seconds': 0}


def assert_calculation(row, expected, block=None):
    result = row['calculation']['result']
    if expected:
        assert result['blocks'] == [], result['blocks']
        assert result['due_at'] == instant(expected), (expected, result)
        outcome = result['arithmetic']['outcome']
        if outcome['kind'] == 'civil_candidate':
            assert outcome['date'] == expected[:10]
    else:
        assert result['due_at'] is None and result['blocks'], result
        if block:
            assert any(value['kind'] == block for value in result['blocks']), result
    assert result['arithmetic'] is None or isinstance(result['arithmetic']['trace'], list)


def exact(saved, record):
    row = record['row']
    collection = route(saved['scenario']['case_id']) + '/deadlines'
    assert request('GET', collection + '/' + row['id'] + '/revisions/1') == row
    history = request('GET', collection + '/' + row['id'] + '/history')
    assert len(history['revisions']) == 1 and not history['has_more']
    assert_calculation(row, record['expected'], record.get('block'))
    if record['kind'] == 'oral':
        pair = record['pair']
        assert prepare_compound(record['draft']['command']) == {'state': 'replay', 'record': pair}
        assert submit_compound(record['draft']) == pair
        result = pair['result']
        result_path = route(saved['scenario']['case_id']) + '/hearings/' + result['hearing_id']
        assert request('GET', result_path + '/results/' + result['id'] + '/revisions/1') == result
    else:
        submit(collection, record['draft'], expected=409, code='deadline_operation_conflict')


def agenda(saved):
    query = {'from': '2026-01-01T00:00:00Z', 'until': '2026-12-01T00:00:00Z',
             'kind': 'deadline', 'limit': 100}
    rows, cursors = [], set()
    for _ in range(16):
        page = request('GET', '/api/v1/agenda?' + urlencode(query))
        rows.extend(row for row in page['items'] if row['kind'] == 'deadline'
                    and row['deadline']['case_id'] == saved['scenario']['case_id'])
        if page['complete']:
            assert page['next_cursor'] is None
            break
        cursor = page['next_cursor']
        assert cursor and cursor not in cursors
        cursors.add(cursor)
        query['cursor'] = cursor
    else:
        raise AssertionError('Corpus agenda exceeded the bounded scan')
    expected = {record['row']['id']: instant(record['expected'])
                for record in saved['records'] if record['expected']}
    assert {row['deadline']['id']: row['at'] for row in rows} == expected
    for row in rows:
        assert row['deadline']['operational']['freshness'] == 'current'
        assert row['deadline']['operational']['due_at'] == row['at']
    collection = request('GET', route(saved['scenario']['case_id']) + '/deadlines?limit=100')
    assert not collection['has_more']
    assert len(collection['deadlines']) == len(saved['records'])


def record_deadline(saved, kind, variant='positive', expected=None, block=None):
    command = source_deadline(saved['scenario'], kind, variant)
    collection = route(saved['scenario']['case_id']) + '/deadlines'
    draft = prepare(collection, command)
    row = submit(collection, draft)
    record = {'kind': kind, 'variant': variant, 'draft': draft, 'row': row,
              'expected': expected, 'block': block}
    saved['records'].append(record)
    save(saved)
    exact(saved, record)



def checkpoint_hearing_agenda(hearing_id):
    path = WORK / 'hearing-api-state.json'
    if not path.exists():
        return
    saved = json.loads(path.read_text(encoding='utf-8'))
    routes = [key for key in saved['records'] if key.startswith('/api/v1/hearings?')]
    assert len(routes) == 1
    route = routes[0]
    previous = saved['records'][route]
    current = request('GET', route)
    before = {row['id']: row for row in previous['hearings']}
    after = {row['id']: row for row in current['hearings']}
    assert all(after.get(identifier) == row for identifier, row in before.items())
    assert set(after) - set(before) <= {hearing_id}
    assert not previous['has_more'] and not current['has_more']
    saved['records'][route] = current
    path.write_text(json.dumps(saved, sort_keys=True), encoding='utf-8')


def capture():
    assert not STATE.exists(), 'Use verify or restore for the existing capture'
    saved = {'scenario': fixture(), 'records': []}
    preferences = request('GET', '/api/v1/alert-preferences')['preferences']
    values = deepcopy(preferences['values'])
    values['deadline_upcoming'] = {'lead_hours': [720],
                                   'channels': {'internal': True, 'email': False}}
    for value in values.values():
        if isinstance(value, dict) and 'channels' in value:
            value['channels']['email'] = False
    request('PUT', '/api/v1/alert-preferences', {'operation_id': str(uuid4()),
            'expected_revision': preferences['revision'], 'values': values})
    save(saved)
    assert set(saved['scenario']['profiles']) == set(EXPECTED) - {'written_holiday'}
    for kind in EXPECTED:
        if kind in ['oral', 'written_holiday']:
            continue
        record_deadline(saved, kind, expected=EXPECTED[kind])
    command = compound(saved['scenario'])
    draft = prepare_compound(command)
    pair = submit_compound(draft)
    saved['records'].append({'kind': 'oral', 'variant': 'positive', 'draft': draft,
                             'pair': pair, 'row': pair['deadline'], 'expected': EXPECTED['oral']})
    save(saved)
    exact(saved, saved['records'][-1])
    checkpoint_hearing_agenda(pair['result']['hearing_id'])
    record_deadline(saved, 'written', 'written_holiday', EXPECTED['written_holiday'])
    for kind, variant, block in [
        ('initial', 'missing_duration', 'rule'),
        ('initial', 'missing_anchor', 'arithmetic'),
        ('initial', 'monthly_no_homologue', 'arithmetic'),
        ('extension', 'monthly_grant_missing', 'condition_rejected'),
        ('extension', 'excess_accumulated', 'condition_rejected'),
        ('written', 'missing_effect', 'trigger'),
        ('control_order', 'scope_rejected', 'scope_rejected'),
        ('control_sentence', 'condition_rejected', 'condition_rejected'),
        ('withdrawal', 'condition_rejected', 'condition_rejected'),
        ('trial_sentence', 'condition_rejected', 'condition_rejected'),
    ]:
        record_deadline(saved, kind, variant, block=block)
    reconcile(saved, 'capture')


def alerts(saved):
    records = {record['row']['id']: record for record in saved['records']}
    wanted = {identifier for identifier, record in records.items() if record['expected']
              and instant(record['expected'])['unix_seconds'] <= time.time() + 720 * 3600}
    expires = time.monotonic() + 15
    while True:
        rows, cursor = [], None
        for _ in range(8):
            query = {'limit': 100, 'read': 'all', 'state': 'active'}
            if cursor:
                query['cursor'] = cursor
            page = request('GET', '/api/v1/alerts?' + urlencode(query))
            rows.extend(row for row in page['alerts'] if row['subject']['kind'] == 'deadline'
                        and row['subject']['id'] in records
                        and row['kind']['kind'] in ['upcoming', 'overdue_unattended'])
            if not page['has_more']:
                break
            cursor = page['next_cursor']
        observed = {row['subject']['id'] for row in rows}
        assert all(records[identifier]['expected'] for identifier in observed)
        if wanted <= observed:
            break
        assert time.monotonic() < expires, ('Missing operational alerts', wanted - observed)
        time.sleep(0.2)
    for row in rows:
        record = records[row['subject']['id']]
        assert row['origin'] == {'revision': 1,
                                'evidence_digest': record['row']['receipt']['capture_digest']}
        field = 'activity_at' if row['kind']['kind'] == 'upcoming' else 'due_at'
        assert row['kind'][field] == instant(record['expected'])
    return [{'id': row['id'], 'subject': row['subject'], 'origin': row['origin'],
             'kind': row['kind']} for row in rows]


def reconcile(saved, mode):
    scenario = saved['scenario']
    assert request('GET', '/api/v1/auth/me')['id'] == scenario['actor']['id']
    for profile in scenario['profiles'].values():
        assert request('GET', route(scenario['case_id']) + '/deadline-profiles/'
                       + profile['id'] + '/revisions/1') == profile
    for record in saved['records']:
        exact(saved, record)
    agenda(saved)
    observed_alerts = alerts(saved)
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Legal corpus accepted: ' + json.dumps({
        'mode': mode, 'case_id': scenario['case_id'], 'covered': list(scenario['profiles']),
        'alerts': observed_alerts,
        'observations': [{'kind': row['kind'], 'variant': row['variant'],
                          'expected': row['expected'], 'result': row['row']['calculation']['result']}
                         for row in saved['records']],
    }, sort_keys=True))


if __name__ == '__main__':
    assert len(sys.argv) == 2 and sys.argv[1] in ['capture', 'verify', 'restore']
    capture() if sys.argv[1] == 'capture' else reconcile(
        json.loads(STATE.read_text(encoding='ascii')), sys.argv[1])
