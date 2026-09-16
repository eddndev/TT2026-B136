"""Bounded HTTP helpers for the disposable procedural fact restore campaign."""
import copy
import json
import os
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen
from uuid import uuid4

BASE = os.environ['TT_FACT_API_BASE_URL']
TOKEN = os.environ['TT_FACT_API_TOKEN']
WORK = Path(os.environ['TT_FACT_API_WORK_DIR'])
REPO = Path(os.environ['TT_FACT_API_REPO'])
STATE = WORK / 'procedural-facts-api-state.json'


def request(method, path, body=None, expected=200, token=TOKEN, code=None, headers=None):
    metadata = {'Authorization': 'Bearer ' + token}
    if isinstance(body, dict):
        body = json.dumps(body, ensure_ascii=True, separators=(',', ':')).encode('ascii')
        metadata['Content-Type'] = 'application/json'
    metadata.update(headers or {})
    query = Request(BASE + path, data=body, headers=metadata, method=method)
    try:
        response = urlopen(query, timeout=60)
    except HTTPError as error:
        response = error
    with response:
        raw = response.read(2 * 1024 * 1024 + 1)
        assert len(raw) <= 2 * 1024 * 1024, 'Fact HTTP response exceeds fixture budget'
        result = json.loads(raw) if raw else None
        actual = (result or {}).get('error', {}).get('code')
        assert response.status == expected, (method, path, response.status, expected, actual)
        if code:
            assert actual == code, (path, actual, code)
    return result


def enroll(role):
    email = 'procedural-facts-' + role + '@example.com'
    password = 'synthetic procedural fact fixture password'
    enrollment = request('POST', '/api/v1/users', {'email': email, 'password': password, 'role': role}, 201)
    challenge = request('POST', '/api/v1/auth/login', {'email': email, 'password': password})
    session = request('POST', '/api/v1/auth/mfa/recovery', {
        'challenge_token': challenge['challenge_token'], 'code': enrollment['recovery_codes'][0],
    })
    return enrollment['user']['id'], session['access_token']


def new_case(label):
    result = request('POST', '/api/v1/cases', {
        'title': 'Declared facts ' + label, 'reference': 'FACTS-' + label,
    }, 201)
    return '/api/v1/cases/' + result['id']


def upload(route, kind, name):
    relative = ('crates/infrastructure/tests/fixtures/stage-support.pdf' if kind == 'pdf'
                else 'crates/infrastructure/src/document_formats/docx/tests/fixtures/producer.docx')
    result = request('POST', route + '/documents', (REPO / relative).read_bytes(), 201,
                     headers={'X-Document-Name': name})
    return {'document_id': result['id'], 'version': result['version'], 'digest': result['digest']}


def evidence(support, locator):
    return {**support, 'locator': locator}


def external(support, locator):
    return {'kind': 'external_reference', 'reference': 'Synthetic declared source',
            'support': evidence(support, locator)}


def values(family):
    rows = json.loads((REPO / 'crates/domain/tests/fixtures/procedural_fact_vectors.json').read_text())
    return copy.deepcopy(next(row['normalized'] for row in rows if row['name'] == family + '_minimum'))


def record(family, value, parent=None):
    result = {'family': family, 'operation_id': str(uuid4()), 'id': str(uuid4()),
              'change': {'action': 'record', 'expected_revision': 0, 'values': value}}
    if family == 'notification':
        assert parent is not None
        result['resolution_id'] = parent
    return result


def base(route, command):
    path = route + '/resolutions'
    if command['family'] == 'notification':
        path += '/' + command['resolution_id'] + '/notifications'
    return path


def prepare(route, command, token=TOKEN, expected=200, code=None):
    return request('POST', base(route, command) + '/prepare', command, expected, token, code)


def submit(route, draft, token=TOKEN, expected=201, code=None):
    command = draft['command']
    action = command['change']['action']
    path = base(route, command)
    if action != 'record':
        path += '/' + command['id']
    if action == 'withdraw':
        path += '/withdrawal'
    return request('PUT' if action == 'correct' else 'POST', path,
                   {'command': command, 'expected_submission_digest': draft['submission_digest']},
                   expected, token, code)


def change(row, action, replacement=None):
    command = {'family': row['family'], 'id': row['id'], 'operation_id': str(uuid4()),
               'change': {'action': action, 'expected_revision': row['revision'],
                          'reason': 'Explicit synthetic declaration ' + action}}
    if row['family'] == 'notification':
        command['resolution_id'] = row['resolution_id']
    if action == 'correct':
        command['change']['values'] = copy.deepcopy(replacement or row['values'])
    return command


def exact_paths(route, row):
    path = base(route, row) + '/' + row['id']
    return [path, path + '/history'] + [path + '/revisions/' + str(r) for r in range(1, row['revision'] + 1)]


def receipt(row, draft):
    command = draft['command']
    assert row['case_id'] == draft['case_id']
    assert row['family'] == command['family'] and row['id'] == command['id']
    assert row['recorded_by']['id'] == draft['actor_id']
    assert row['revision'] == draft['result_revision']
    assert row['receipt']['operation_id'] == command['operation_id']
    assert row['receipt']['action'] == command['change']['action']
    assert row['receipt']['expected_revision'] == command['change']['expected_revision']
    assert row['receipt']['sources_digest'] == draft['sources_digest']
    assert row['receipt']['submission_digest'] == draft['submission_digest']
    if row['family'] == 'notification':
        assert row['resolution_id'] == command['resolution_id']
