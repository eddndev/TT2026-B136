#!/usr/bin/env python3
"""Exercise real serve reevaluation using independent explicit Follow fixtures."""
import copy
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from uuid import UUID, uuid4
from api_deadlines_support import correct, prepare, profile, register, request, resolution, submit

STATE = Path(os.environ['TT_DEADLINE_API_WORK_DIR']) / 'deadline-reevaluation-state.json'
TIMEOUT = 60


def persist(route, command, method='POST', suffix=''):
    draft = request('POST', route + '/prepare', command)
    return request(method, route + suffix, {
        'command': draft['command'], 'expected_submission_digest': draft['submission_digest'],
    }, 201)


def instant(iso):
    return {'unix_seconds': int(datetime.fromisoformat(iso).timestamp()),
            'nanosecond': 0, 'offset_seconds': 0}


def await_revision(path, revision):
    until = time.monotonic() + TIMEOUT
    while time.monotonic() < until:
        row = request('GET', path)
        assert row['revision'] <= revision, 'Unexpected duplicate or unrelated revision'
        if row['revision'] == revision:
            return row
        time.sleep(0.2)
    raise AssertionError('Reevaluation did not reach revision ' + str(revision))


def historical(row):
    result = copy.deepcopy(row)
    result['operational'] = {'freshness': 'not_checked', 'checked_at': None,
                             'changed_dependencies': [], 'due_at': None}
    return result


def check_technical(row, previous, source, family, review):
    assert row['receipt']['action'] == 'reevaluate'
    assert row['recorded_by'] == {
        'kind': 'technical', 'service': 'deadline_reevaluator', 'policy_version': 1}
    version = row['receipt']['version']
    assert version['kind'] == 'v2'
    assert version['predecessor'] == {
        'submission_digest': previous['receipt']['submission_digest'],
        'capture_digest': previous['receipt']['capture_digest']}
    cause = version['cause']
    assert cause['kind'] == 'source_event'
    UUID(cause['job_id'])
    event = cause['event']
    assert event['family'] == family and event['source_id'] == source['id']
    assert event['revision'] == source['revision']
    assert event['operation_id'] == source['receipt']['operation_id']
    assert isinstance(event['sequence'], str) and event['sequence'].isdigit()
    assert 0 < int(event['sequence']) <= 9223372036854775807
    assert row['tracking']['review']['state'] == review


def new_calendar():
    source = str(uuid4())
    values = {
        'scope': {'title': 'Worker arithmetic calendar', 'jurisdiction': 'local',
            'authority': 'Synthetic authority', 'organ': 'Synthetic office',
            'territory': 'Declared territory', 'entity_codes': ['09'],
            'use_description': 'Synthetic arithmetic without legal applicability'},
        'coverage': {'from': '2026-01-01', 'through': '2026-03-31'},
        'sources': [{'id': source, 'title': 'Synthetic calendar reference', 'issuer': 'Fixture',
            'official_url': 'https://example.org/worker-calendar', 'published_on': None,
            'consulted_on': '2026-01-01', 'locator': 'Arithmetic example'}],
        'weekly_pattern': [{'weekday': day, 'classification': 'countable',
            'source_ids': [source], 'explanation': 'Declared countable fixture day'}
            for day in range(1, 8)], 'exceptions': [],
    }
    return persist('/api/v1/judicial-calendars', {
        'calendar_id': str(uuid4()), 'operation_id': str(uuid4()),
        'change': {'action': 'publish', 'expected_revision': 0, 'values': values}})


def replace_calendar(row, values):
    return persist('/api/v1/judicial-calendars', {
        'calendar_id': row['id'], 'operation_id': str(uuid4()),
        'change': {'action': 'replace', 'expected_revision': row['revision'],
            'values': values, 'reason': 'Explicit synthetic calendar revision'}},
        'PUT', '/' + row['id'])


def capture():
    owner = request('GET', '/api/v1/auth/me')
    case = request('POST', '/api/v1/cases', {
        'title': 'Serve worker acceptance', 'reference': 'WORKER-' + str(uuid4())}, 201)
    route = '/api/v1/cases/' + case['id']
    source = resolution(route)
    calendar = new_calendar()
    base_profile = profile(case['id'], 'daily')
    definition = copy.deepcopy(base_profile['definition'])
    definition['template']['rule']['basis'] = 'calendar_countable'
    definition['examples'][0]['calendar'] = copy.deepcopy(calendar['values'])
    selected_profile = persist(route + '/deadline-profiles', {
        'profile_id': base_profile['id'], 'operation_id': str(uuid4()),
        'change': {'action': 'replace', 'expected_revision': base_profile['revision'],
            'definition': definition, 'reason': 'Declare countable-day fixture'}},
        'PUT', '/' + base_profile['id'])
    command = register(case['id'], selected_profile, source, owner['id'])
    command['change']['tracking'] = {'profile': 'follow', 'source': 'follow', 'calendar': 'follow'}
    command['change']['definition']['input']['calendar'] = {'id': calendar['id'], 'revision': 1}
    base = route + '/deadlines'
    first = submit(base, prepare(base, command))
    path = base + '/' + first['id']
    initial_due = instant('2026-01-31T23:30:00+00:00')
    assert request('GET', path)['operational']['due_at'] == initial_due
    values = copy.deepcopy(calendar['values'])
    values['exceptions'] = [{'id': str(uuid4()), 'from': '2026-01-31', 'through': '2026-01-31',
        'classification': 'excluded', 'source_ids': [values['sources'][0]['id']],
        'explanation': 'Exclude the initial anchor for this arithmetic fixture'}]
    calendar = replace_calendar(calendar, values)
    second = await_revision(path, 2)
    check_technical(second, first, calendar, 'calendar', 'accepted')
    assert second['tracking']['review']['reasons'] == []
    assert second['definition']['input']['calendar'] == {'id': calendar['id'], 'revision': 2}
    changed_due = instant('2026-02-01T23:30:00+00:00')
    assert second['calculation']['result']['due_at'] == changed_due
    assert second['operational']['freshness'] == 'current'
    assert second['operational']['due_at'] == changed_due
    values = copy.deepcopy(source['values'])
    values['issued_at'] = {'precision': 'date', 'year': 2026, 'month': 2,
                           'day': 2, 'offset_seconds': None}
    source = persist(route + '/resolutions', {'family': 'resolution', 'id': source['id'],
        'operation_id': str(uuid4()), 'change': {'action': 'correct', 'expected_revision': 1,
            'values': values, 'reason': 'Change declared source date'}}, 'PUT', '/' + source['id'])
    third = await_revision(path, 3)
    check_technical(third, second, source, 'resolution', 'pending')
    assert third['calculation'] == second['calculation']
    assert third['definition'] == second['definition']
    assert third['tracking']['review']['reasons'] == [
        {'dependency': 'source', 'reason': 'source_changed'}]
    assert third['operational']['due_at'] is None
    assert third['operational']['freshness'] == 'current'
    command = correct(third)
    command['change']['definition']['input']['selection']['source']['value']['revision'] = 2
    command['change']['reason'] = 'Human review explicitly accepts the current declared source'
    draft = prepare(base, command)
    assert draft['author'] == {'kind': 'user', 'id': owner['id'], 'email': owner['email']}
    fourth = submit(base, draft)
    assert fourth['revision'] == 4 and fourth['tracking']['review']['state'] == 'accepted'
    assert fourth['receipt']['version']['cause'] is None
    assert fourth['operational']['freshness'] == 'not_checked'
    assert request('GET', path)['operational']['due_at'] == instant('2026-02-02T23:30:00+00:00')
    rows = [historical(row) for row in [first, second, third, fourth]]
    for row in rows:
        assert request('GET', path + '/revisions/' + str(row['revision'])) == row
    STATE.write_text(json.dumps({'path': path, 'calendar': calendar, 'rows': rows}, sort_keys=True))
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Serve reevaluation capture passed: calendar movement, source review, human acceptance, exact history.')


def restarted():
    saved = json.loads(STATE.read_text())
    path, rows = saved['path'], saved['rows']
    for row in rows:
        assert request('GET', path + '/revisions/' + str(row['revision'])) == row
    assert request('GET', path)['revision'] == 4
    values = copy.deepcopy(saved['calendar']['values'])
    values['sources'][0]['locator'] = 'Post-restart witness with unchanged countable days'
    calendar = replace_calendar(saved['calendar'], values)
    witness = await_revision(path, 5)
    check_technical(witness, rows[-1], calendar, 'calendar', 'accepted')
    assert witness['calculation']['result']['due_at'] == rows[-1]['calculation']['result']['due_at']
    assert witness['operational']['due_at'] == rows[-1]['calculation']['result']['due_at']
    history = request('GET', path + '/history?limit=20')['revisions']
    assert [row['revision'] for row in history] == [5, 4, 3, 2, 1]
    assert len({row['receipt']['operation_id'] for row in history}) == 5
    for row in rows:
        assert request('GET', path + '/revisions/' + str(row['revision'])) == row
    saved['rows'].append(historical(witness))
    saved['calendar'] = calendar
    STATE.write_text(json.dumps(saved, sort_keys=True))
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Serve restart acceptance passed: witness consumed, no duplicate revision, exact historical evidence.')


def verify():
    saved = json.loads(STATE.read_text())
    for row in saved['rows']:
        assert request('GET', saved['path'] + '/revisions/' + str(row['revision'])) == row
    current = request('GET', saved['path'])
    assert current['revision'] == len(saved['rows'])
    assert historical(current) == saved['rows'][-1]
    assert current['operational']['due_at'] == saved['rows'][-1]['calculation']['result']['due_at']
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Serve read-only restart verification passed: exact history and operational head retained.')


if __name__ == '__main__':
    if os.environ.get('TT_DEADLINE_REEVALUATION_ACCEPTANCE') != '1':
        raise SystemExit('Explicit disposable acceptance opt-in is required')
    {'capture': capture, 'restarted': restarted, 'verify': verify}[sys.argv[1]]()
