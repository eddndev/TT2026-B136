#!/usr/bin/env python3
"""Exercise global calendar revisions, civil classifications and exact HTTP restore."""
import copy
import json
import os
from pathlib import Path
import sys
from urllib.error import HTTPError
from urllib.request import Request, urlopen
from uuid import uuid4

BASE = os.environ['TT_CALENDAR_API_BASE_URL']
TOKEN = os.environ['TT_CALENDAR_API_TOKEN']
WORK = Path(os.environ['TT_CALENDAR_API_WORK_DIR'])
REPO = Path(os.environ['TT_CALENDAR_API_REPO'])
ROUTE = '/api/v1/judicial-calendars'
STATE = WORK / 'calendar-api-state.json'


def request(method, path, body=None, expected=200, token=TOKEN, code=None):
    headers = {'Authorization': 'Bearer ' + token}
    if body is not None:
        body = json.dumps(body, ensure_ascii=True, separators=(',', ':')).encode('ascii')
        headers['Content-Type'] = 'application/json'
    query = Request(BASE + path, data=body, headers=headers, method=method)
    try:
        response = urlopen(query, timeout=60)
    except HTTPError as error:
        response = error
    with response:
        raw = response.read(2 * 1024 * 1024)
        result = json.loads(raw) if raw else None
        actual = (result or {}).get('error', {}).get('code')
        assert response.status == expected, (method, path, response.status, expected, actual)
        if code:
            assert actual == code, (path, actual, code)
    return result


def enroll(role):
    email = 'calendar-' + role + '@example.com'
    password = 'synthetic calendar fixture password'
    enrollment = request('POST', '/api/v1/users', {'email': email, 'password': password, 'role': role}, 201)
    challenge = request('POST', '/api/v1/auth/login', {'email': email, 'password': password})
    session = request('POST', '/api/v1/auth/mfa/recovery', {
        'challenge_token': challenge['challenge_token'], 'code': enrollment['recovery_codes'][0],
    })
    return enrollment['user']['id'], session['access_token']


def publish(values):
    return {'operation_id': str(uuid4()), 'calendar_id': str(uuid4()),
            'change': {'action': 'publish', 'expected_revision': 0, 'values': values}}


def prepare(command, token=TOKEN, expected=200, code=None):
    return request('POST', ROUTE + '/prepare', command, expected, token, code)


def submit(draft, token=TOKEN, expected=201, code=None):
    command = draft['command']
    action = command['change']['action']
    path = ROUTE if action == 'publish' else ROUTE + '/' + command['calendar_id']
    if action == 'retire':
        path += '/retirement'
    return request('PUT' if action == 'replace' else 'POST', path,
                   {'command': command, 'expected_submission_digest': draft['submission_digest']},
                   expected, token, code)


def capture():
    fixtures = json.loads((REPO / 'crates/domain/tests/fixtures/judicial_calendar_vectors.json').read_text())
    _, litigator = enroll('litigator')
    _, paralegal = enroll('paralegal')
    _, client = enroll('client')
    other_id, other_owner = enroll('owner')
    assert request('GET', ROUTE)['calendars'] == []
    command = publish(fixtures[3]['input'])
    command['calendar_id'] = '00000000-0000-0000-0000-000000000000'
    draft = prepare(command)
    assert draft['command']['change']['values'] == fixtures[3]['normalized']
    assert draft['values_digest'] == fixtures[3]['sha256']
    assert request('GET', ROUTE)['calendars'] == []
    for token in [litigator, paralegal, client]:
        prepare(command, token, 403)
        submit(draft, token, 403)
    first = submit(draft)
    path = ROUTE + '/' + first['id']
    assert first['receipt']['submission_digest'] == draft['submission_digest']
    assert request('GET', path + '/revisions/1') == first
    for token in [litigator, paralegal]:
        assert request('GET', path, token=token) == first
        assert len(request('GET', ROUTE, token=token)['calendars']) == 1
    request('GET', path, expected=403, token=client)
    request('GET', ROUTE, expected=403, token=client)
    request('GET', path + '/revisions/2', expected=404, code='judicial_calendar_not_found')
    days_path = path + '/revisions/1/days?from=2000-02-26&through=2000-03-05'
    days = request('GET', days_path)['days']
    by_date = {day['date']: day for day in days}
    assert len(days) == 9 and by_date['2000-02-26']['state'] == 'outside_coverage'
    assert by_date['2000-02-28']['state'] == 'countable'
    assert by_date['2000-02-29']['origin'] == 'exception'
    assert by_date['2000-02-29']['state'] == 'excluded'
    assert by_date['2000-03-02']['state'] == 'unresolved'
    for invalid in [ROUTE + '?limit=0', path + '?revision=1', days_path + '&revision=2']:
        request('GET', invalid, expected=400, code='invalid_query')
    replacement = {'operation_id': str(uuid4()), 'calendar_id': first['id'], 'change': {
        'action': 'replace', 'expected_revision': 1, 'values': copy.deepcopy(first['values']),
        'reason': 'Rechecked declared classification',
    }}
    replacement['change']['values']['weekly_pattern'][0]['explanation'] = 'Reviewed source locator'
    stale = prepare(replacement)
    competing = copy.deepcopy(replacement)
    competing['operation_id'] = str(uuid4())
    second = submit(prepare(competing, other_owner), other_owner)
    assert second['revision'] == 2 and second['recorded_by']['id'] == other_id
    submit(stale, expected=409, code='judicial_calendar_revision_conflict')
    invalid_scope = copy.deepcopy(replacement)
    invalid_scope['change']['expected_revision'] = 2
    invalid_scope['change']['values']['scope']['title'] = 'Different scope'
    prepare(invalid_scope, expected=422, code='judicial_calendar_scope_change_forbidden')
    retirement = {'operation_id': str(uuid4()), 'calendar_id': first['id'],
                  'change': {'action': 'retire', 'expected_revision': 2, 'reason': 'Retired catalog entry'}}
    third = submit(prepare(retirement))
    assert third['revision'] == 3 and third['status'] == 'retired'
    assert third['values'] == second['values'] and third['values_digest'] == second['values_digest']
    assert request('GET', ROUTE)['calendars'] == []
    terminal = copy.deepcopy(replacement)
    terminal['change']['expected_revision'] = 3
    prepare(terminal, expected=409, code='judicial_calendar_retired')
    reused = publish(first['values'])
    reused['operation_id'] = first['receipt']['operation_id']
    prepare(reused, expected=409, code='judicial_calendar_operation_conflict')
    maximum = submit(prepare(publish(fixtures[-1]['normalized'])))
    assert maximum['values_digest'] == fixtures[-1]['sha256']
    history = request('GET', path + '/history?limit=2')
    assert [item['revision'] for item in history['revisions']] == [3, 2] and history['has_more']
    assert request('GET', path + '/history?before_revision=2')['revisions'][0]['revision'] == 1
    pages, after = [], ''
    while True:
        page = request('GET', ROUTE + '?status=all&limit=1' + after)
        pages.extend(row['id'] for row in page['calendars'])
        if not page['has_more']:
            break
        after = '&after_id=' + page['next_after_id']
        assert len(pages) <= 2
    assert pages == [first['id'], maximum['id']]
    paths = [ROUTE + '?status=all', ROUTE + '?entity_code=09', path, path + '/history',
             path + '/revisions/1', path + '/revisions/2', path + '/revisions/3', days_path,
             path + '/revisions/3/days?from=2000-02-26&through=2000-03-05',
             ROUTE + '/' + maximum['id'] + '/revisions/1']
    STATE.write_text(json.dumps({'records': {p: request('GET', p) for p in paths}}, sort_keys=True), encoding='utf-8')
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Calendar API passed: global permissions, exact receipts, civil dates, conflicts, retirement and maximum Unicode.')


def checkpoint_collections():
    """Capture mutable catalog lists after other fixtures publish calendars."""
    saved = json.loads(STATE.read_text(encoding='utf-8'))
    for path in saved['records']:
        if path.split('?', 1)[0] == ROUTE:
            saved['records'][path] = request('GET', path)
    STATE.write_text(json.dumps(saved, sort_keys=True), encoding='utf-8')
    print('Calendar catalog checkpoint refreshed before backup; exact revision expectations retained.')


def restore():
    saved = json.loads(STATE.read_text(encoding='utf-8'))
    for path, expected in saved['records'].items():
        assert request('GET', path) == expected, 'Restored calendar response differs: ' + path
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Calendar restore passed: 10 exact responses, immutable values, civil days and operation receipts.')


if __name__ == '__main__':
    {'capture': capture, 'checkpoint': checkpoint_collections, 'restore': restore}[sys.argv[1]]()
