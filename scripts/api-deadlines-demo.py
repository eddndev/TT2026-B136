#!/usr/bin/env python3
"""Exercise persistent deadlines through the real API and restored database."""
import copy
import json
import sys
from uuid import uuid4
from api_deadlines_support import (
    STATE, TOKEN, correct, enroll, prepare, profile, qualified_time,
    register, request, resolution, submit,
)


def capture():
    cases = [request('POST', '/api/v1/cases', {'title': 'Deadlines ' + name,
        'reference': 'DEADLINES-' + name}, 201)['id'] for name in ['MAIN', 'FOREIGN']]
    route = '/api/v1/cases/' + cases[0]
    base = route + '/deadlines'
    foreign = '/api/v1/cases/' + cases[1] + '/deadlines'
    users, tokens = {}, {}
    for role in ['litigator', 'paralegal', 'client']:
        users[role], tokens[role] = enroll(role)
        request('PUT', route + '/members/' + users[role], expected=204)
    source = resolution(route)
    daily = profile(cases[0], 'daily')
    monthly = profile(cases[0], 'monthly')
    hourly = profile(cases[0], 'hourly')
    command = register(cases[0], daily, source, users['litigator'])
    assert request('GET', base)['deadlines'] == []
    draft = prepare(base, command, token=tokens['litigator'])
    assert draft['command'] == command
    assert request('GET', base)['deadlines'] == []
    for role in ['paralegal', 'client']:
        prepare(base, command, token=tokens[role], expected=403)
        submit(base, draft, token=tokens[role], expected=403)
    request('GET', base, token=tokens['client'], expected=403)
    submit(base, {**draft, 'submission_digest': '00' * 32}, token=tokens['litigator'],
        expected=409, code='deadline_submission_mismatch')
    first = submit(base, draft, token=tokens['litigator'])
    path = base + '/' + first['id']
    assert first['receipt']['submission_digest'] == draft['submission_digest']
    assert first['calculation'] == draft['calculation']
    assert first['calculation']['result']['blocks'] == []
    assert first['calculation']['result']['arithmetic']['outcome']['date'] == '2026-01-31'
    assert first['calculation']['result']['due_at'] is not None
    assert request('GET', path, token=tokens['paralegal']) == first
    request('GET', foreign + '/' + first['id'], expected=404)
    request('GET', foreign, token=tokens['litigator'], expected=404)
    request('GET', path + '/revisions/99', expected=404, code='deadline_not_found')
    change = correct(first)
    change['change']['definition']['title'] = 'Corrected declared title'
    stale = prepare(base, change)
    winner = copy.deepcopy(change)
    winner['operation_id'] = str(uuid4())
    second = submit(base, prepare(base, winner))
    submit(base, stale, expected=409, code='deadline_revision_conflict')
    attention = {'operation_id': str(uuid4()), 'deadline_id': first['id'], 'change': {
        'action': 'set_attention', 'expected_revision': 2, 'reason': 'Record declared filing',
        'attention': {'status': 'recorded', 'occurred_at': {'precision': 'unknown'},
            'statement': 'Declared filing without inferring legal validity', 'locator': 'Filing record'}}}
    third = submit(base, prepare(base, attention))
    fourth = submit(base, prepare(base, {'operation_id': str(uuid4()), 'deadline_id': first['id'],
        'change': {'action': 'retire', 'expected_revision': 3, 'reason': 'Retire declared tracking'}}))
    assert fourth['status'] == 'retired'
    assert second['calculation'] == third['calculation'] == fourth['calculation']
    assert third['attention'] == fourth['attention']
    prepare(base, correct(fourth), expected=409, code='deadline_retired')
    history = request('GET', path + '/history?limit=2')
    assert [row['revision'] for row in history['revisions']] == [4, 3] and history['has_more']
    assert all('calculation' not in row for row in history['revisions'])
    assert request('GET', path + '/history?before_revision=3')['revisions'][0]['revision'] == 2
    assert request('GET', path + '/revisions/1') == first

    month = submit(base, prepare(base, register(cases[0], monthly, source, users['litigator'])))
    result = month['calculation']['result']
    assert result['due_at'] is None
    assert result['arithmetic']['outcome'] == {'kind': 'blocked', 'block': {
        'kind': 'missing_homologous_day', 'year': 2026, 'month': 2, 'requested_day': 31}}
    command = register(cases[0], hourly, source, users['litigator'])
    command['change']['definition']['input']['selection']['qualification'] = {
        'purpose': 'ordered_period_start', 'at': qualified_time(),
        'statement': 'Declared period start', 'locator': 'Resolution page 1'}
    missing = submit(base, prepare(base, command))
    assert missing['calculation']['result']['rule'] is None
    assert missing['calculation']['result']['due_at'] is None
    change = correct(missing)
    change['change']['definition']['input']['ordered_quantity'] = 24
    hours = submit(base, prepare(base, change))
    result = hours['calculation']['result']
    assert result['due_at'] == {'unix_seconds': 1767817807, 'nanosecond': 0, 'offset_seconds': 0}
    assert result['arithmetic']['trace'][0]['quantity'] == 24
    assert result['arithmetic']['anchor']['offset_seconds'] == -21600
    assert request('GET', base + '/' + missing['id'] + '/revisions/1') == missing

    # Revocation leaves captured responsibility and historical calculations intact.
    request('DELETE', route + '/members/' + users['litigator'], expected=204)
    request('GET', path, token=tokens['litigator'], expected=404)
    prepare(base, correct(hours), expected=409, code='deadline_responsible_unavailable')
    request('GET', path, token=tokens['paralegal'])
    paths = [base + '?status=all']
    for record in [fourth, month, hours]:
        prefix = base + '/' + record['id']
        paths.extend([prefix, prefix + '/history'])
        paths.extend(prefix + '/revisions/' + str(revision) for revision in range(1, record['revision'] + 1))
    records = {p: request('GET', p) for p in paths}
    STATE.write_text(json.dumps({'records': records}, sort_keys=True))
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Deadline API passed: daily/monthly/hourly, four roles, isolation, review, conflicts, attention, retirement and revocation.')


def restore():
    saved = json.loads(STATE.read_text())
    for path, expected in saved['records'].items():
        assert request('GET', path) == expected, 'Restored deadline differs: ' + path
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Deadline restore passed: ' + str(len(saved['records'])) + ' exact responses and captured results.')


if __name__ == '__main__':
    {'capture': capture, 'restore': restore}[sys.argv[1]]()
