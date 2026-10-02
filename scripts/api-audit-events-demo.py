#!/usr/bin/env python3
"""Compare bounded Owner audit reads with real stored events before and after restore."""
from datetime import datetime, timedelta, timezone
import json
import os
import subprocess
import sys
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen
from api_procedural_facts_support import BASE, TOKEN, WORK, request

ROOT = '/api/v1/audit/events'
STATE = WORK / 'audit-events-api-state.json'
FIELDS = {'sequence', 'timestamp', 'actor', 'action', 'resource'}


def exchange(query, token=TOKEN, status=200, code=None):
    suffix = query if isinstance(query, str) else urlencode(query)
    headers = {'Authorization': 'Bearer ' + token} if token else {}
    try:
        response = urlopen(Request(BASE + ROOT + '?' + suffix, headers=headers), timeout=60)
    except HTTPError as error:
        response = error
    with response:
        data = response.read(2 * 1024 * 1024 + 1)
        assert len(data) <= 2 * 1024 * 1024, 'Audit response exceeds fixture budget'
        value = json.loads(data)
        assert response.status == status, (response.status, status, value)
        assert response.headers.get('Cache-Control') == 'no-store'
        if code:
            assert value == {'error': value['error']}
            assert value['error']['code'] == code, value
        return value


def stored(resource, actor):
    # psql quotes variables as SQL literals; all fixture writes still use HTTP.
    sql = """BEGIN READ ONLY;
    SELECT jsonb_build_object('sequence',sequence::text,'timestamp',timestamp,
      'actor',actor,'action',action,'resource',resource,'chain',encode(chain,'hex'))
    FROM audit_events WHERE resource=:'resource' AND actor=:'actor' AND action='case.read'
    ORDER BY timestamp_seconds,timestamp_nanos,sequence;
    COMMIT;
    """
    result = subprocess.run(['psql', os.environ['TT_AUDIT_DATABASE_URL'], '-XqAt',
                             '-v', 'ON_ERROR_STOP=1', '-v', 'resource=' + resource,
                             '-v', 'actor=' + actor], input=sql, text=True,
                            capture_output=True, timeout=30, check=True)
    return [json.loads(line) for line in result.stdout.splitlines() if line]


def public(rows):
    return [{key: row[key] for key in FIELDS} for row in rows]


def validate(page, query, snapshot=None):
    assert set(page) == {'checked_at', 'snapshot_max_sequence', 'events', 'has_more', 'next_cursor'}
    assert isinstance(page['snapshot_max_sequence'], str) and page['snapshot_max_sequence'].isdigit()
    assert int(page['snapshot_max_sequence']) <= 2**63 - 1
    if snapshot is not None:
        assert page['snapshot_max_sequence'] == snapshot, 'Continuation changed its snapshot'
    assert isinstance(page['checked_at'], str) and page['checked_at'].endswith('Z')
    assert len(page['events']) <= int(query['limit'])
    for row in page['events']:
        assert set(row) == FIELDS, 'Audit read invented historical identity or network fields'
        assert isinstance(row['sequence'], str) and row['sequence'].isdigit()
        assert int(row['sequence']) <= int(page['snapshot_max_sequence'])
        for name in ['actor', 'action', 'resource']:
            if name in query:
                assert row[name] == query[name]
    assert page['has_more'] == (page['next_cursor'] is not None)
    if page['has_more']:
        assert page['events'] and 0 < len(page['next_cursor']) <= 4096


def collect(query, first=None):
    page = first if first is not None else exchange(query)
    snapshot, rows, seen = page['snapshot_max_sequence'], [], set()
    for _ in range(8):
        validate(page, query, snapshot)
        for row in page['events']:
            assert row['sequence'] not in seen, 'Audit continuation duplicated a historical event'
            seen.add(row['sequence'])
            rows.append(row)
        if not page['has_more']:
            return rows
        page = exchange({**query, 'cursor': page['next_cursor']})
    raise AssertionError('Audit fixture exceeded its page budget')


def capture():
    actor = request('GET', '/api/v1/auth/me')['email']
    case = request('POST', '/api/v1/cases', {
        'title': 'Audit historical event fixture', 'reference': 'AUDIT-HISTORY-HTTP',
    }, 201)
    route = '/api/v1/cases/' + case['id']
    for _ in range(3):
        request('GET', route)
    resource = 'case:' + case['id']
    before = stored(resource, actor)
    assert len(before) == 3
    now = datetime.now(timezone.utc)
    query = {'from': (now - timedelta(days=1)).strftime('%Y-%m-%dT00:00:00Z'),
             'until': (now + timedelta(days=2)).strftime('%Y-%m-%dT00:00:00Z'),
             'actor': actor, 'action': 'case.read', 'resource': resource, 'limit': 1}
    first = exchange(query)
    validate(first, query)
    assert first['events'] == public(before[:1]) and first['has_more']
    request('GET', route)
    after = stored(resource, actor)
    assert after[:3] == before and len(after) == 4, 'Reading rewrote earlier audit history'
    assert int(after[-1]['sequence']) > int(first['snapshot_max_sequence'])
    assert collect(query, first) == public(before), 'Continuation admitted a later append'
    assert collect(query) == public(after), 'Fresh query omitted the later append'
    assert before[0]['timestamp'] != before[1]['timestamp']
    exact = {**query, 'from': before[0]['timestamp'], 'until': before[1]['timestamp'], 'limit': 100}
    assert exchange(exact)['events'] == public(before[:1]), 'Exact time bounds lost precision'
    empty = exchange({**query, 'resource': resource + ':no-such-resource'})
    assert empty['events'] == [] and not empty['has_more'] and empty['next_cursor'] is None
    exchange({**query, 'actor': actor.upper(), 'cursor': first['next_cursor']},
             status=400, code='invalid_audit_query')
    for suffix in ['&limit=2', '&unknown=1', '&actor=%FF', '&resource=%']:
        exchange(urlencode(query) + suffix, status=400, code='invalid_audit_query')
    exchange(query, token=None, status=401, code='invalid_session')
    exchange(query, token='not-a-real-session', status=401, code='invalid_session')
    tokens = json.loads((WORK / 'deadlines-api-state.json').read_text())['agenda']['tokens']
    for role in ['litigator', 'paralegal', 'client']:
        exchange(query, token=tokens[role], status=403, code='permission_denied')
        exchange({**query, 'resource': resource + ':no-such-resource'},
                 token=tokens[role], status=403, code='permission_denied')
    assert stored(resource, actor) == after, 'Audit querying changed its source events'
    assert request('GET', '/api/v1/audit/verify')['valid'], 'Existing chain verification failed'
    value = {'query': query, 'before': before, 'after': after, 'first': first}
    with os.fdopen(os.open(STATE, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), 'w') as stream:
        json.dump(value, stream, ensure_ascii=True, sort_keys=True)
    print('Owner audit capture passed: exact stored history, time bounds, snapshot pages, '
          'later append isolation, strict query, three denied roles and existing chain verification.')


def restore():
    saved = json.loads(STATE.read_text())
    query = saved['query']
    assert stored(query['resource'], query['actor']) == saved['after'], 'Restored historical bytes changed'
    assert collect(query) == public(saved['after'])
    assert collect(query, saved['first']) == public(saved['before']), 'Restored cursor changed its cut'
    assert stored(query['resource'], query['actor']) == saved['after']
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Owner audit restore passed: exact timestamp/text/sequence/chain and stable continuation.')


if __name__ == '__main__':
    {'capture': capture, 'restore': restore}[sys.argv[1]]()
