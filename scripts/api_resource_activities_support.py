"""Exact association fixtures over the shared disposable HTTP server."""
import copy
import json
from uuid import uuid4
import api_procedural_facts_support as facts
import api_procedural_resources_support as resources

TOKEN, WORK, request = facts.TOKEN, facts.WORK, facts.request
STATE = WORK / 'resource-activities-api-state.json'


def targets():
    records = json.loads((WORK / 'hearing-api-state.json').read_text())['records']
    for row in records.values():
        if not isinstance(row, dict) or 'scheduling_context' not in row or row['revision'] < 2:
            continue
        route = '/api/v1/cases/' + row['case_id']
        if request('GET', route + '/administration')['administration']['administrative_status'] != 'active':
            continue
        hearing = request('GET', route + '/hearings/' + row['id'] + '/revisions/1')
        break
    else:
        raise AssertionError('Expected an active case with a historical hearing')
    fixture = json.loads((WORK / 'deadlines-api-state.json').read_text())['agenda']
    deadline = request('GET', '/api/v1/cases/' + fixture['case_id'] + '/deadlines/'
                       + fixture['retired_id'] + '/revisions/1')
    return hearing, deadline


def own_resource(target, with_act=False):
    route = '/api/v1/cases/' + target['case_id']
    support = facts.evidence(facts.upload(route, 'pdf', 'association-source.pdf'), 'Page 1')
    value = facts.values('resolution')
    value['summary'] = 'Independent synthetic association source'
    value['provenance'] = facts.external(support, 'Page 1')
    parent = facts.submit(route, facts.prepare(route, facts.record('resolution', value)))
    first = resources.submit(route, resources.prepare(route, resources.registration(parent, support)))
    if not with_act:
        return first, first, None
    act = resources.submit(route, resources.prepare(route, resources.change(
        first, 'record_act', resources.act_values('oral', support))))
    replacement = copy.deepcopy(act['act']['values'])
    replacement['statement'] = 'Later correction preserves the selected historical act'
    head = resources.submit(route, resources.prepare(route, resources.change(
        act, 'correct_act', replacement, act['act'])))
    return first, head, act


def enroll(role, route):
    email = 'resource-activities-' + role + '@example.com'
    password = 'synthetic resource activities fixture password'
    user = request('POST', '/api/v1/users', {'email': email, 'password': password, 'role': role}, 201)
    challenge = request('POST', '/api/v1/auth/login', {'email': email, 'password': password})
    session = request('POST', '/api/v1/auth/mfa/recovery', {
        'challenge_token': challenge['challenge_token'], 'code': user['recovery_codes'][0],
    })
    request('PUT', route + '/members/' + user['user']['id'], expected=204)
    return user['user']['id'], session['access_token']


def base(row):
    return '/api/v1/cases/' + row['case_id'] + '/procedural-resources/' + row['id'] + '/activities'


def link(first, head, target, kind, act=None):
    reference = {'kind': kind, 'id': target['id'], 'revision': target['revision']}
    digest = 'submission_digest' if kind == 'hearing' else 'capture_digest'
    reference[digest] = target['receipt'][digest]
    selected_act = None if act is None else {
        'id': act['act']['id'], 'revision': act['act']['revision'],
        'resource_revision': act['revision'], 'capture_digest': act['receipt']['capture_digest'],
    }
    return {'case_id': first['case_id'], 'resource_id': first['id'],
            'association_id': str(uuid4()), 'operation_id': str(uuid4()),
            'expected_resource_revision': head['revision'], 'change': {
                'action': 'link', 'expected_revision': 0,
                'resource': {'id': first['id'], 'revision': first['revision'],
                             'capture_digest': first['receipt']['capture_digest']},
                'act': selected_act, 'target': reference}}


def unlink(row, head):
    return {'case_id': row['case_id'], 'resource_id': row['resource_id'],
            'association_id': row['id'], 'operation_id': str(uuid4()),
            'expected_resource_revision': head['revision'], 'change': {
                'action': 'unlink', 'expected_revision': row['revision'],
                'reason': 'Organizational association withdrawal'}}


def prepare(path, command, token=TOKEN, expected=200, code=None):
    return request('POST', path + '/prepare', command, expected, token, code)


def submit(path, draft, token=TOKEN, expected=201, code=None):
    command = draft['command']
    if command['change']['action'] == 'unlink':
        path += '/' + command['association_id'] + '/unlink'
    return request('POST', path, {'command': command,
                   'expected_submission_digest': draft['submission_digest']}, expected, token, code)


def receipt(row, draft):
    command = draft['command']
    assert row['case_id'] == command['case_id'] and row['resource_id'] == command['resource_id']
    assert row['id'] == command['association_id'] and row['revision'] == draft['result_revision']
    for field in ['selection', 'status', 'sources', 'recorded_by']:
        assert row[field] == draft[field], field
    assert row['recorded_administration'] == draft['observed_administration']
    assert row['recorded_resource_head'] == draft['observed_resource_head']
    for field in ['operation_id', 'expected_resource_revision']:
        assert row['receipt'][field] == command[field]
    for field in ['action', 'expected_revision']:
        assert row['receipt'][field] == command['change'][field]
    assert row['receipt']['previous'] == draft['previous']
    assert row['receipt']['submission_digest'] == draft['submission_digest']
    assert len(row['receipt']['capture_digest']) == 64
    assert 'current_target' not in row


def instant(value):
    assert set(value) == {'unix_seconds', 'nanosecond', 'offset_seconds'}
    assert type(value['unix_seconds']) is int and value['unix_seconds'] > 0
    assert type(value['nanosecond']) is int and 0 <= value['nanosecond'] < 1_000_000_000
    assert value['offset_seconds'] == 0


def stable(value):
    """Validate common read instants before normalizing only those transient fields."""
    if isinstance(value, list):
        return [stable(item) for item in value]
    if not isinstance(value, dict):
        return value
    out = copy.deepcopy(value)
    if 'associations' in out and out['associations']:
        assert all(v['checked_at'] == out['associations'][0]['checked_at'] for v in out['associations'])
    if 'current_target' in out:
        instant(out['checked_at'])
        current = out['current_target']
        if current['kind'] == 'deadline':
            checked = current['record']['operational']['checked_at']
            assert checked is None or checked == out['checked_at']
        out['checked_at'] = 'validated association read instant'
    if 'operational' in out:
        op = out['operational']
        if op['freshness'] == 'not_checked':
            assert op['checked_at'] is None
        else:
            instant(op['checked_at'])
            op['checked_at'] = 'validated deadline read instant'
    return {key: stable(item) for key, item in out.items()}
