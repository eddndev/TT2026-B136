#!/usr/bin/env python3
"""Verify the combined agenda using existing disposable hearing/deadline records."""
import json
import os
from pathlib import Path
from urllib.parse import urlencode
from uuid import UUID

from api_deadlines_support import STATE, TOKEN, request


RANGE = {'from': '2026-01-01T00:00:00Z', 'until': '2027-01-01T00:00:00Z',
         'kind': 'all', 'hearing_status': 'all'}


def endpoint(params):
    return '/api/v1/agenda?' + urlencode(params)


def identity(row):
    return row['kind'], row[row['kind']]['id']


def key(row):
    at = row['at']
    assert at['offset_seconds'] == 0
    assert 0 <= at['nanosecond'] < 1_000_000_000
    return (at['unix_seconds'], at['nanosecond'],
            0 if row['kind'] == 'hearing' else 1, UUID(identity(row)[1]).int)


def page(params, token=TOKEN):
    value = request('GET', endpoint(params), token=token)
    assert set(value) == {'from', 'until', 'kind', 'hearing_status',
                          'checked_at', 'items', 'complete', 'next_cursor'}
    for field in RANGE:
        assert value[field] == params[field]
    checked = value['checked_at']
    assert isinstance(checked['unix_seconds'], int) and checked['offset_seconds'] == 0
    assert 0 <= checked['nanosecond'] < 1_000_000_000
    assert len(value['items']) <= params.get('limit', 20)
    assert value['complete'] == (value['next_cursor'] is None)
    if not value['complete']:
        assert value['next_cursor'] != params.get('cursor')
    keys = [key(row) for row in value['items']]
    assert keys == sorted(set(keys))
    assert len({identity(row) for row in value['items']}) == len(keys)
    for row in value['items']:
        if row['kind'] != 'deadline':
            assert 'venue' not in row['hearing'] and 'note' not in row['hearing']
            continue
        deadline = row['deadline']
        assert deadline['status'] == 'active' and deadline['receipt_kind'] == 'v2'
        assert deadline['review_state'] == 'accepted' and not deadline['calculation_blocked']
        assert deadline['operational']['freshness'] == 'current'
        assert deadline['operational']['checked_at'] == checked
        assert row['at'] == deadline['operational']['due_at'] == deadline['calculation_due_at']
    return value


def verify():
    fixture = json.loads(STATE.read_text())['agenda']
    work = Path(os.environ['TT_DEADLINE_API_WORK_DIR'])
    hearings = json.loads((work / 'hearing-api-state.json').read_text())['records']
    hearing_rows = next(value['hearings'] for path, value in hearings.items()
                        if path.startswith('/api/v1/hearings?'))
    full = page({**RANGE, 'limit': 100})
    assert full['complete']
    identities = [identity(row) for row in full['items']]
    expected_hearings = {('hearing', row['id']) for row in hearing_rows}
    assert expected_hearings <= set(identities)
    assert ('deadline', fixture['deadline_id']) in identities
    assert not {('deadline', fixture['retired_id']), ('deadline', fixture['blocked_id'])} & set(identities)

    params = {**RANGE, 'limit': 1}
    collected, cursors = [], []
    for _ in range(32):
        value = page(params)
        collected.extend(value['items'])
        if value['complete']:
            break
        cursor = value['next_cursor']
        assert cursor not in cursors
        cursors.append(cursor)
        params['cursor'] = cursor
    else:
        raise AssertionError('Combined agenda exceeded the fixture page budget')
    assert cursors, 'The mixed fixture must exercise continuation'
    assert [identity(row) for row in collected] == identities
    assert [key(row) for row in collected] == [key(row) for row in full['items']]
    request('GET', endpoint({**RANGE, 'kind': 'hearing', 'cursor': cursors[0]}), expected=400)

    for kind, status in [('hearing', 'all'), ('hearing', 'scheduled'), ('deadline', 'scheduled')]:
        filtered = page({**RANGE, 'kind': kind, 'hearing_status': status, 'limit': 100})
        assert filtered['complete']
        expected = [row for row in full['items'] if row['kind'] == kind
                    and (kind == 'deadline' or status == 'all' or row['hearing']['status'] == status)]
        assert [identity(row) for row in filtered['items']] == [identity(row) for row in expected]

    staff = page({**RANGE, 'limit': 100}, fixture['tokens']['paralegal'])
    assert staff['complete']
    assert [identity(row) for row in staff['items']] == [('deadline', fixture['deadline_id'])]
    assert staff['items'][0]['deadline']['case_id'] == fixture['case_id']
    revoked = page({**RANGE, 'limit': 100}, fixture['tokens']['litigator'])
    assert revoked['complete'] and revoked['items'] == []
    denied = request('GET', endpoint(RANGE), token=fixture['tokens']['client'],
                     expected=403, code='permission_denied')
    empty_period = {**RANGE, 'from': '2035-01-01T00:00:00Z', 'until': '2035-01-02T00:00:00Z'}
    assert request('GET', endpoint(empty_period), token=fixture['tokens']['client'],
                   expected=403, code='permission_denied') == denied
    assert set(denied) == {'error'} and 'items' not in denied
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Combined agenda API passed: existing mixed records, exact cursor order, filters, '
          'current-only deadlines, assigned staff, revoked membership and non-disclosing Client denial.')


if __name__ == '__main__':
    verify()
