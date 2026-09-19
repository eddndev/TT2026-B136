"""Exact resource commands over the existing disposable HTTP fixture server."""
import copy
import json
from uuid import uuid4
from api_procedural_facts_support import TOKEN, WORK, request

STATE = WORK / 'procedural-resources-api-state.json'


def enroll(role):
    email = 'procedural-resources-' + role + '@example.com'
    password = 'synthetic procedural resource fixture password'
    enrollment = request('POST', '/api/v1/users', {
        'email': email, 'password': password, 'role': role,
    }, 201)
    challenge = request('POST', '/api/v1/auth/login', {'email': email, 'password': password})
    session = request('POST', '/api/v1/auth/mfa/recovery', {
        'challenge_token': challenge['challenge_token'], 'code': enrollment['recovery_codes'][0],
    })
    return enrollment['user']['id'], session['access_token']


def fixtures():
    records = json.loads((WORK / 'procedural-facts-api-state.json').read_text())['records']
    for row in records.values():
        if not isinstance(row, dict) or row.get('family') != 'resolution':
            continue
        if row.get('status') != 'withdrawn' or not row.get('sources', {}).get('direct_supports'):
            continue
        route = '/api/v1/cases/' + row['case_id']
        if request('GET', route + '/administration')['administration']['administrative_status'] != 'active':
            continue
        original = request('GET', route + '/resolutions/' + row['id'] + '/revisions/1')
        support = original['values']['provenance']['support']
        assert support and support['version'] == 1
        candidates = [source for value in records.values() if isinstance(value, dict)
                      and value.get('case_id') == row['case_id']
                      for source in value.get('sources', {}).get('direct_supports', [])]
        docx = next(source for source in candidates if source['format'] == 'docx')
        participant = next(source for value in records.values() if isinstance(value, dict)
                           and value.get('case_id') == row['case_id']
                           for source in value.get('sources', {}).get('participants', []))
        return route, original, copy.deepcopy(support), {
            key: docx[key] for key in ['document_id', 'version', 'digest']
        } | {'locator': 'Historical oral act constancy paragraph 1'}, participant
    raise AssertionError('Existing fact capture must contain an active case and historical supports')


def unknown(reason='Not stated in the synthetic source'):
    return {'kind': 'unknown', 'reason': reason}


def values(parent, support, kind):
    return {
        'kind': kind, 'mode': {'kind': 'known', 'value': 'written'},
        'title': 'Declared ' + kind + ' resource',
        'resolution': {'id': parent['id'], 'revision': parent['revision']},
        'resolution_evidence': copy.deepcopy(support), 'resolution_reference': unknown(),
        'issuing_authority': unknown(), 'receiving_authority': None,
        'resolution_at': copy.deepcopy(parent['values']['issued_at']), 'notification_at': None,
        'challenged_part': 'Declared challenged paragraph', 'grounds': 'Declared resource reasons',
        'appellants': [{'name': 'Captured recurrent person', 'role': unknown(), 'participant': None}],
    }


def registration(parent, support, kind='revocation'):
    return {'operation_id': str(uuid4()), 'resource_id': str(uuid4()),
            'change': {'action': 'register', 'expected_revision': 0,
                       'values': values(parent, support, kind)}}


def change(row, action, replacement=None, act=None):
    value = {'action': action, 'expected_revision': row['revision']}
    if action in ['correct', 'correct_act', 'archive', 'reactivate']:
        value['reason'] = 'Explicit synthetic correction or organizational change'
    if action == 'correct':
        value['values'] = copy.deepcopy(replacement or row['values'])
    if action in ['record_act', 'correct_act']:
        value['act_id'] = act['id'] if act else str(uuid4())
        value['values'] = copy.deepcopy(replacement)
        if action == 'correct_act':
            value['expected_act_revision'] = act['revision']
    return {'operation_id': str(uuid4()), 'resource_id': row['id'], 'change': value}


def act_values(mode, support):
    return {'kind': 'interposition', 'mode': {'kind': 'known', 'value': mode},
            'occurred_at': {'precision': 'date', 'year': 2026, 'month': 9, 'day': 19,
                            'offset_seconds': None},
            'authority': unknown(), 'statement': 'Declared ' + mode + ' presentation',
            'evidence': [copy.deepcopy(support)]}


def base(route):
    return route + '/procedural-resources'


def prepare(route, command, token=TOKEN, expected=200, code=None):
    return request('POST', base(route) + '/prepare', command, expected, token, code)


def submit(route, draft, token=TOKEN, expected=201, code=None):
    command = draft['command']
    action = command['change']['action']
    path = base(route)
    method = 'POST'
    if action != 'register':
        path += '/' + command['resource_id']
    if action in ['correct', 'correct_act']:
        method = 'PUT'
    if action in ['record_act', 'correct_act']:
        path += '/acts'
        if action == 'correct_act':
            path += '/' + command['change']['act_id']
    if action in ['archive', 'reactivate']:
        path += '/archive' if action == 'archive' else '/reactivation'
    return request(method, path, {'command': command,
                   'expected_submission_digest': draft['submission_digest']}, expected, token, code)


def receipt(row, draft):
    command = draft['command']
    assert row['case_id'] == draft['case_id'] and row['id'] == command['resource_id']
    assert row['revision'] == draft['result_revision']
    for field in ['values', 'status', 'sources', 'act', 'recorded_by']:
        assert row[field] == draft[field], field
    assert row['recorded_administration'] == draft['observed_administration']
    assert row['recorded_stage'] == draft['observed_stage']
    assert row['receipt']['operation_id'] == command['operation_id']
    assert row['receipt']['action'] == command['change']['action']
    assert row['receipt']['expected_revision'] == command['change']['expected_revision']
    assert row['receipt']['previous'] == draft['previous']
    assert row['receipt']['submission_digest'] == draft['submission_digest']
    assert len(row['receipt']['capture_digest']) == 64


def exact_paths(route, row):
    path = base(route) + '/' + row['id']
    return [path, path + '/history'] + [path + '/revisions/' + str(n)
                                      for n in range(1, row['revision'] + 1)]
