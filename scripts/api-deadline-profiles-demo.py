#!/usr/bin/env python3
"""Reproduce profile catalog authorization, receipts and restoration over real HTTP."""
import copy
import json
import os
from pathlib import Path
import sys
from urllib.error import HTTPError
from urllib.request import Request, urlopen
from uuid import uuid4

BASE = os.environ['TT_PROFILE_API_BASE_URL']
TOKEN = os.environ['TT_PROFILE_API_TOKEN']
STATE = Path(os.environ['TT_PROFILE_API_WORK_DIR']) / 'deadline-profiles-api-state.json'
ROUTE = '/api/v1/deadline-profiles'


def request(method, path, body=None, expected=200, token=TOKEN, code=None):
    headers = {'Authorization': 'Bearer ' + token}
    if body is not None:
        body = json.dumps(body, ensure_ascii=True, separators=(',', ':')).encode('ascii')
        headers['Content-Type'] = 'application/json'
    try:
        response = urlopen(Request(BASE + path, data=body, headers=headers, method=method), timeout=60)
    except HTTPError as error:
        response = error
    with response:
        raw = response.read(2 * 1024 * 1024 + 1)
        assert len(raw) <= 2 * 1024 * 1024, 'Profile response exceeds fixture budget'
        value = json.loads(raw) if raw else None
        actual = (value or {}).get('error', {}).get('code')
        assert response.status == expected, (method, path, response.status, expected, actual)
        if code:
            assert actual == code, (path, actual, code)
        return value


def enroll(role):
    email = 'deadline-profile-' + role + '@example.com'
    password = 'synthetic profile fixture password'
    enrollment = request('POST', '/api/v1/users', {'email': email, 'password': password, 'role': role}, 201)
    challenge = request('POST', '/api/v1/auth/login', {'email': email, 'password': password})
    session = request('POST', '/api/v1/auth/mfa/recovery', {
        'challenge_token': challenge['challenge_token'], 'code': enrollment['recovery_codes'][0],
    })
    return enrollment['user']['id'], session['access_token']


def definition(case_id=None):
    ref = str(uuid4())
    scope = {'kind': 'case', 'case_id': case_id} if case_id else {'kind': 'global', 'value': {
        'title': 'Declared scope', 'jurisdiction': 'federal', 'entity_codes': ['09'],
        'authority': 'Synthetic authority', 'organ': 'Synthetic organ',
        'territory': 'Declared territory', 'use_description': 'Mathematical fixture',
    }}
    return {
        'title': 'Synthetic daily profile', 'description': 'Arithmetic fixture without legal claims',
        'scope': scope, 'references': [{'id': ref, 'title': 'Synthetic reference', 'issuer': 'Fixture',
            'official_url': 'https://example.org/synthetic', 'published_on': None,
            'consulted_on': '2026-01-06', 'locator': 'Mathematical example'}],
        'trigger': {'kind': 'source_field', 'field': 'resolution_issued_at'},
        'template': {'kind': 'fixed', 'rule': {'kind': 'days', 'quantity': 1,
            'inclusion': 'on_anchor', 'basis': 'natural', 'final_day': 'preserve'}},
        'completion': {'kind': 'civil_candidate_only'},
        'conditions': [{'id': str(uuid4()), 'statement': 'Operator declares applicability', 'reference_ids': [ref]}],
        'examples': [{'id': str(uuid4()), 'anchor': {'precision': 'date', 'year': 2026,
            'month': 1, 'day': 6, 'offset_seconds': None}, 'ordered_quantity': None, 'calendar': None,
            'expected': {'kind': 'arithmetic', 'outcome': {'kind': 'civil_candidate', 'date': '2026-01-06'}},
            'reference_ids': [ref], 'locator': 'One included natural day'}],
    }


def publish(case_id=None):
    return {'profile_id': str(uuid4()), 'operation_id': str(uuid4()),
            'change': {'action': 'publish', 'expected_revision': 0, 'definition': definition(case_id)}}


def prepare(base, command, token=TOKEN, expected=200, code=None):
    return request('POST', base + '/prepare', command, expected, token, code)


def submit(base, draft, token=TOKEN, expected=201, code=None):
    command = draft['command']
    action = command['change']['action']
    path = base if action == 'publish' else base + '/' + command['profile_id']
    if action == 'retire':
        path += '/retirement'
    return request('PUT' if action == 'replace' else 'POST', path,
                   {'command': command, 'expected_submission_digest': draft['submission_digest']},
                   expected, token, code)


def replacement(row):
    return {'profile_id': row['id'], 'operation_id': str(uuid4()), 'change': {
        'action': 'replace', 'expected_revision': row['revision'],
        'definition': copy.deepcopy(row['definition']), 'reason': 'Reviewed synthetic example',
    }}


def capture():
    cases = [request('POST', '/api/v1/cases', {'title': 'Profile ' + name,
              'reference': 'PROFILE-' + name}, 201)['id'] for name in ['MAIN', 'FOREIGN']]
    private = '/api/v1/cases/' + cases[0] + '/deadline-profiles'
    foreign = '/api/v1/cases/' + cases[1] + '/deadline-profiles'
    users, tokens = {}, {}
    for role in ['litigator', 'paralegal', 'client']:
        users[role], tokens[role] = enroll(role)
        request('PUT', '/api/v1/cases/' + cases[0] + '/members/' + users[role], expected=204)
    assert request('GET', ROUTE)['profiles'] == []
    command = publish()
    command['profile_id'] = '00000000-0000-0000-0000-000000000000'
    draft = prepare(ROUTE, command)
    assert draft['command'] == command and draft['algorithm'] == 'v1'
    assert request('GET', ROUTE)['profiles'] == []
    for token in tokens.values():
        prepare(ROUTE, command, token, 403)
        submit(ROUTE, draft, token, 403)
    submit(ROUTE, {**draft, 'submission_digest': '00' * 32}, expected=409,
           code='deadline_profile_submission_mismatch')
    first = submit(ROUTE, draft)
    path = ROUTE + '/' + first['id']
    assert first['receipt']['submission_digest'] == draft['submission_digest']
    assert first['definition'] == draft['definition']
    assert request('GET', path + '/revisions/1') == first
    local = submit(private, prepare(private, publish(cases[0])))
    local_path = private + '/' + local['id']
    assert len(request('GET', ROUTE)['profiles']) == 1
    assert len(request('GET', private)['profiles']) == 2
    assert len(request('GET', foreign)['profiles']) == 1
    request('GET', ROUTE + '/' + local['id'], expected=404)
    request('GET', foreign + '/' + local['id'], expected=404)
    for role in ['litigator', 'paralegal']:
        assert request('GET', path, token=tokens[role]) == first
        assert request('GET', local_path, token=tokens[role]) == local
        request('GET', foreign, expected=404, token=tokens[role])
        prepare(private, publish(cases[0]), tokens[role], 403)
    for base in [ROUTE, private]:
        request('GET', base, expected=403, token=tokens['client'])
    prepare(private, replacement(first), expected=400, code='deadline_profile_command_mismatch')
    wrong_scope = replacement(first)
    wrong_scope['change']['definition']['scope']['value']['title'] = 'Different scope'
    prepare(ROUTE, wrong_scope, expected=409, code='deadline_profile_scope_change_forbidden')
    changed = replacement(first)
    changed['change']['definition']['description'] = 'Reviewed arithmetic fixture'
    stale = prepare(ROUTE, changed)
    winner = copy.deepcopy(changed)
    winner['operation_id'] = str(uuid4())
    second = submit(ROUTE, prepare(ROUTE, winner))
    assert second['revision'] == 2
    submit(ROUTE, stale, expected=409, code='deadline_profile_revision_conflict')
    invalid = replacement(second)
    invalid['change']['definition']['examples'][0]['expected']['outcome']['date'] = '2026-01-07'
    prepare(ROUTE, invalid, expected=422, code='deadline_profile_example_mismatch')
    retired = submit(ROUTE, prepare(ROUTE, {'profile_id': first['id'], 'operation_id': str(uuid4()),
        'change': {'action': 'retire', 'expected_revision': 2, 'reason': 'Retired synthetic rule'}}))
    assert retired['revision'] == 3 and retired['status'] == 'retired'
    assert retired['definition'] == second['definition']
    prepare(ROUTE, replacement(retired), expected=409, code='deadline_profile_retired')
    assert request('GET', ROUTE)['profiles'] == []
    history = request('GET', path + '/history?limit=2')
    assert [row['revision'] for row in history['revisions']] == [3, 2] and history['has_more']
    assert all('definition' not in row for row in history['revisions'])
    assert request('GET', path + '/history?before_revision=2')['revisions'][0]['revision'] == 1
    request('DELETE', '/api/v1/cases/' + cases[0] + '/members/' + users['litigator'], expected=204)
    request('GET', local_path, expected=404, token=tokens['litigator'])
    assert request('GET', path, token=tokens['litigator']) == retired
    paths = [ROUTE + '?status=all', private + '?status=all', path, path + '/history',
             path + '/revisions/1', path + '/revisions/2', path + '/revisions/3',
             local_path, local_path + '/history', local_path + '/revisions/1']
    STATE.write_text(json.dumps({'records': {p: request('GET', p) for p in paths}}, sort_keys=True))
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Profile API passed: four roles, global/private isolation, corpus, receipts, stale writes, retirement and revocation.')


def restore():
    saved = json.loads(STATE.read_text())
    for path, expected in saved['records'].items():
        assert request('GET', path) == expected, 'Restored profile differs: ' + path
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Profile restore passed: 10 exact responses and immutable receipts.')


if __name__ == '__main__':
    {'capture': capture, 'restore': restore}[sys.argv[1]]()
