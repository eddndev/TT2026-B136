#!/usr/bin/env python3
"""Verify the three agenda families using existing disposable activity records."""
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
from urllib.parse import urlencode
from uuid import UUID

from api_deadlines_support import STATE, TOKEN, request


RANGE = {'from': '2026-01-01T00:00:00Z', 'until': '2027-01-01T00:00:00Z',
         'kind': 'all', 'hearing_status': 'all'}
FAMILY_RANK = {'hearing': 0, 'deadline': 1, 'resource_hearing': 2}


def endpoint(params):
    return '/api/v1/agenda?' + urlencode(params)


def identity(row):
    assert row['kind'] in FAMILY_RANK
    return row['kind'], row[row['kind']]['id']


def canonical_uuid(value):
    assert isinstance(value, str)
    try:
        parsed = UUID(value)
    except ValueError as error:
        raise AssertionError('Agenda identifier is not a UUID') from error
    assert str(parsed) == value
    return parsed


def instant(value):
    assert set(value) == {'unix_seconds', 'nanosecond', 'offset_seconds'}
    assert type(value['unix_seconds']) is int and value['offset_seconds'] == 0
    assert -62135596800 <= value['unix_seconds'] <= 253402300799
    assert type(value['nanosecond']) is int and 0 <= value['nanosecond'] < 1_000_000_000
    return value['unix_seconds'], value['nanosecond']


def scheduled_time(value):
    assert isinstance(value, str)
    assert re.fullmatch(r'[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}'
                        r'(Z|[+-][0-9]{2}:[0-9]{2})', value)
    assert not value.endswith('-00:00')
    try:
        parsed = datetime.fromisoformat(value.replace('Z', '+00:00'))
        assert abs(parsed.utcoffset().total_seconds()) <= 14 * 3600
        utc = parsed.astimezone(timezone.utc)
    except (ValueError, OverflowError) as error:
        raise AssertionError('Agenda date is not a representable instant') from error
    return int((utc - datetime(1970, 1, 1, tzinfo=timezone.utc)).total_seconds()), 0


def key(row):
    seconds, nanos = instant(row['at'])
    kind, identifier = identity(row)
    return seconds, nanos, FAMILY_RANK[kind], canonical_uuid(identifier).int


def selected(row, kind, status):
    if kind != 'all' and row['kind'] != kind:
        return False
    if row['kind'] == 'hearing':
        return status == 'all' or row['hearing']['status'] == status
    return row['kind'] != 'resource_hearing' or status != 'cancelled'


def resource_hearing(row):
    assert set(row) == {'kind', 'at', 'case_title', 'case_reference', 'case_status',
                        'resource_hearing'}
    for field in ['case_title', 'case_reference']:
        assert isinstance(row[field], str) and row[field] and row[field].strip() == row[field]
    assert row['case_status'] in {'active', 'closed'}
    hearing = row['resource_hearing']
    assert set(hearing) == {'case_id', 'resource_id', 'id', 'revision', 'kind', 'scheduled_at',
                            'modality', 'participant_count', 'association_id', 'capture_digest'}
    for field in ['case_id', 'resource_id', 'id', 'association_id']:
        canonical_uuid(hearing[field])
    assert type(hearing['revision']) is int and hearing['revision'] == 1
    assert hearing['kind'] in {'appeal_arguments', 'written_revocation'}
    assert hearing['modality'] in {'in_person', 'videoconference'}
    assert type(hearing['participant_count']) is int and 0 <= hearing['participant_count'] <= 32
    assert isinstance(hearing['capture_digest'], str)
    assert re.fullmatch(r'[0-9a-f]{64}', hearing['capture_digest'])
    assert scheduled_time(hearing['scheduled_at']) == instant(row['at'])


def page(params, token=TOKEN):
    value = request('GET', endpoint(params), token=token)
    assert set(value) == {'from', 'until', 'kind', 'hearing_status',
                          'checked_at', 'items', 'complete', 'next_cursor'}
    for field in RANGE:
        assert value[field] == params[field]
    checked = value['checked_at']
    instant(checked)
    start, until = scheduled_time(params['from']), scheduled_time(params['until'])
    assert len(value['items']) <= params.get('limit', 20)
    assert value['complete'] == (value['next_cursor'] is None)
    if not value['complete']:
        assert value['next_cursor'] != params.get('cursor')
    keys = [key(row) for row in value['items']]
    assert keys == sorted(set(keys))
    assert len({identity(row) for row in value['items']}) == len(keys)
    for row in value['items']:
        assert start <= instant(row['at']) < until
        assert selected(row, params['kind'], params['hearing_status'])
        if row['kind'] == 'hearing':
            assert 'venue' not in row['hearing'] and 'note' not in row['hearing']
            assert row['hearing']['status'] in {'scheduled', 'cancelled'}
            continue
        if row['kind'] == 'resource_hearing':
            resource_hearing(row)
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

    for kind, status in [('all', 'scheduled'), ('all', 'cancelled'),
                         ('hearing', 'all'), ('hearing', 'scheduled'), ('hearing', 'cancelled'),
                         ('deadline', 'scheduled'), ('resource_hearing', 'all'),
                         ('resource_hearing', 'scheduled')]:
        filtered = page({**RANGE, 'kind': kind, 'hearing_status': status, 'limit': 100})
        assert filtered['complete']
        expected = [row for row in full['items'] if selected(row, kind, status)]
        assert [identity(row) for row in filtered['items']] == [identity(row) for row in expected]
    request('GET', endpoint({**RANGE, 'kind': 'resource_hearing', 'hearing_status': 'cancelled'}),
            expected=400)

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
    print('Combined agenda API passed: three family projections, exact cursor order, filters, '
          'current-only deadlines, assigned staff, revoked membership and non-disclosing Client denial.')


if __name__ == '__main__':
    verify()
