#!/usr/bin/env python3
"""Exercise account access changes, durable revocation and assignment selectors."""
import json
import sys
from urllib.parse import urlencode
from api_procedural_facts_support import WORK, request

STATE = WORK / 'members-api-state.json'
PASSWORD = 'synthetic member lifecycle password'
PREFIX = 'member-http-'


def enroll(role):
    return request('POST', '/api/v1/users', {
        'email': PREFIX + role + '@example.com', 'password': PASSWORD, 'role': role,
    }, 201)


def challenge(row):
    return request('POST', '/api/v1/auth/login', {'email': row['user']['email'], 'password': PASSWORD})['challenge_token']


def login(row, index):
    return request('POST', '/api/v1/auth/mfa/recovery', {
        'challenge_token': challenge(row), 'code': row['recovery_codes'][index],
    })['access_token']


def user(identifier):
    row = request('GET', '/api/v1/users/' + identifier)
    assert set(row) == {'id', 'email', 'role', 'active', 'revision'}
    assert row['id'] == identifier and row['revision'] == str(int(row['revision']))
    return row


def access(row, role, active, token=None):
    kwargs = {} if token is None else {'token': token}
    result = request('PUT', '/api/v1/users/' + row['id'] + '/access', {
        'expected_revision': row['revision'], 'role': role, 'active': active,
    }, **kwargs)
    assert result == {**row, 'role': role, 'active': active,
                      'revision': str(int(row['revision']) + (row['role'] != role or row['active'] != active))}
    return result


def pages(path, **query):
    rows, cursors = [], set()
    for _ in range(100):
        page = request('GET', path + '?' + urlencode({'limit': 1, 'email_prefix': PREFIX, **query}))
        assert len(page['items']) <= 1
        rows.extend(page['items'])
        if not page['has_more']:
            assert page['next_cursor'] is None
            assert [row['id'] for row in rows] == sorted({row['id'] for row in rows})
            return rows
        cursor = page['next_cursor']
        assert cursor and cursor not in cursors
        cursors.add(cursor)
        query['cursor'] = cursor
    raise AssertionError('Member fixture exceeded its page budget')


def capture():
    enrollments = {role: enroll(role) for role in ['owner', 'litigator', 'paralegal', 'client']}
    tokens = {role: login(row, 0) for role, row in enrollments.items()}
    ids = {role: row['user']['id'] for role, row in enrollments.items()}
    second_token = login(enrollments['litigator'], 1)
    old_challenge = challenge(enrollments['litigator'])
    listing = pages('/api/v1/users', status='all')
    assert {row['id'] for row in listing} == set(ids.values())
    for role in ['litigator', 'paralegal', 'client']:
        for path in ['/api/v1/users', '/api/v1/users/' + ids['owner']]:
            request('GET', path, token=tokens[role], expected=403, code='permission_denied')
    case = request('POST', '/api/v1/cases', {'title': 'Member lifecycle', 'reference': 'MEMBER-HTTP'}, 201)
    route = '/api/v1/cases/' + case['id']
    request('PUT', route + '/members/' + ids['litigator'], expected=204)
    assigned = pages(route + '/members', selection='assigned')
    assert [row['id'] for row in assigned] == [ids['litigator']] and assigned[0]['assigned_at']
    assert {row['id'] for row in pages(route + '/members', selection='available')} == set(ids.values()) - {ids['litigator']}
    before = user(ids['litigator'])
    disabled = access(before, 'litigator', False)
    assert pages(route + '/members', selection='assigned')[0]['active'] is False
    for old in [tokens['litigator'], second_token]:
        request('GET', '/api/v1/auth/me', token=old, expected=401, code='invalid_session')
    enabled = access(disabled, 'litigator', True)
    for old in [tokens['litigator'], second_token]:
        request('GET', '/api/v1/auth/me', token=old, expected=401, code='invalid_session')
    request('POST', '/api/v1/auth/mfa/recovery', {
        'challenge_token': old_challenge, 'code': enrollments['litigator']['recovery_codes'][2],
    }, expected=401, code='mfa_rejected')
    assert user(ids['litigator']) == enabled
    fresh_token = login(enrollments['litigator'], 2)
    request('GET', route, token=fresh_token)
    current = user(ids['litigator'])
    assert access(current, 'litigator', True) == current
    request('GET', '/api/v1/auth/me', token=fresh_token)
    request('PUT', '/api/v1/users/' + ids['litigator'] + '/access', {
        'expected_revision': before['revision'], 'role': 'litigator', 'active': True,
    }, expected=409, code='user_revision_conflict')
    access(current, 'paralegal', True)
    request('GET', '/api/v1/auth/me', token=fresh_token, expected=401, code='invalid_session')
    paralegal_token = login(enrollments['litigator'], 3)
    request('POST', '/api/v1/cases', {'title': 'Denied', 'reference': 'MEMBER-DENIED'},
            expected=403, token=paralegal_token, code='permission_denied')
    own = user(ids['owner'])
    access(own, 'paralegal', True, tokens['owner'])
    request('GET', '/api/v1/auth/me', token=tokens['owner'], expected=401, code='invalid_session')
    access(user(ids['litigator']), 'paralegal', False)
    admin = request('GET', route + '/administration')['administration']
    request('PUT', route + '/administrative-status', {
        'expected_revision': admin['revision'], 'administrative_status': 'closed',
    })
    request('DELETE', route + '/members/' + ids['litigator'], expected=204)
    assert pages(route + '/members', selection='assigned') == []
    assert ids['litigator'] not in {row['id'] for row in pages(route + '/members', selection='available')}
    rows = {identifier: user(identifier) for identifier in ids.values()}
    STATE.write_text(json.dumps({'rows': rows, 'case_id': case['id'], 'old_token': tokens['client']}, sort_keys=True))
    STATE.chmod(0o600)
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Member API passed: filtered directory, assignments, CAS, role/activity, self-change and durable revocation.')


def restore():
    saved = json.loads(STATE.read_text())
    for identifier, row in saved['rows'].items():
        assert user(identifier) == row, 'Restored user projection differs'
    request('GET', '/api/v1/auth/me', token=saved['old_token'], expected=401, code='invalid_session')
    assert pages('/api/v1/users', status='all') == sorted(saved['rows'].values(), key=lambda row: row['id'])
    assert pages('/api/v1/cases/' + saved['case_id'] + '/members', selection='assigned') == []
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Member restore passed: exact access projections and assignments, invalid previous sessions and fresh MFA.')


if __name__ == '__main__':
    assert len(sys.argv) == 2 and sys.argv[1] in ['capture', 'restore']
    capture() if sys.argv[1] == 'capture' else restore()
