#!/usr/bin/env python3
"""Accept native compound hearing results and exact restart/restore reconciliation."""
import copy
import json
import sys
from urllib.parse import urlencode
from uuid import uuid4

from api_hearing_derived_deadline_contract import assert_creation
from api_hearing_derived_deadline_fixtures import (
    TOKEN, WORK, base, changed, command, fixture, login, prepare, request, route, submit,
)

STATE = WORK / 'hearing-derived-deadline-api-state.json'


def save(value):
    STATE.write_text(json.dumps(value, sort_keys=True, ensure_ascii=True), encoding='ascii')
    STATE.chmod(0o600)


def result_path(command):
    return route(command['case_id']) + '/hearings/' + command['result']['hearing_id'] + '/results'


def absent(value):
    request('GET', result_path(value) + '/' + value['result']['result_id'] + '/revisions/1',
            expected=404, code='hearing_result_not_found')
    request('GET', route(value['case_id']) + '/deadlines/' + value['deadline']['deadline_id'],
            expected=404, code='deadline_not_found')


def exact_pair(record):
    value = record['command']
    assert request('GET', result_path(value) + '/' + value['result']['result_id'] + '/revisions/1') == record['result']
    assert request('GET', route(value['case_id']) + '/deadlines/'
                   + value['deadline']['deadline_id'] + '/revisions/1') == record['deadline']


def counts(scenario, records):
    page = request('GET', route(scenario['case_id']) + '/deadlines?limit=100')
    assert not page['has_more'] and page['next_after_id'] is None
    assert {row['id'] for row in page['deadlines']} == {row['deadline']['id'] for row in records}
    assert len(page['deadlines']) == len(records)
    for record in records:
        page = request('GET', result_path(record['command']) + '?limit=100')
        assert not page['has_more'] and page['next_after_id'] is None
        assert len(page['results']) == 1
        assert page['results'][0]['id'] == record['result']['id']
        assert page['results'][0]['revision'] == 1


def agenda(scenario, records):
    query = urlencode({'from': '2026-01-07T00:00:00Z', 'until': '2026-01-08T00:00:00Z',
                       'kind': 'deadline', 'limit': 100})
    page = request('GET', '/api/v1/agenda?' + query)
    assert page['complete'] and page['next_cursor'] is None
    selected = [row for row in page['items'] if row['kind'] == 'deadline'
                and row['deadline']['case_id'] == scenario['case_id']]
    assert len(selected) == 1, 'Only the calculable consequence belongs in the deadline agenda'
    row = selected[0]
    assert row['deadline']['id'] == records[0]['deadline']['id']
    assert row['deadline']['operational']['freshness'] == 'current'
    assert row['deadline']['operational']['checked_at'] == page['checked_at']
    assert row['at'] == row['deadline']['operational']['due_at'] == records[0]['deadline']['calculation']['result']['due_at']
    blocked = request('GET', route(scenario['case_id']) + '/deadlines/' + records[1]['deadline']['id'])
    assert blocked['calculation']['result']['blocks']
    assert blocked['calculation']['result']['due_at'] is None
    assert blocked['operational']['due_at'] is None


def rejected_access(scenario, ready):
    accounts = scenario['accounts']
    for token, status, code in [('', 401, 'invalid_session'),
                               (accounts['paralegal']['token'], 403, 'permission_denied'),
                               (accounts['client']['token'], 403, 'permission_denied'),
                               (accounts['outside']['token'], 404, 'case_not_found')]:
        prepare(ready['command'], token, status, code)
        submit(ready, token, status, code)
    # The routed case must agree with the body before any material is selected.
    foreign = base(ready['command']).replace(scenario['case_id'], str(uuid4()))
    request('POST', foreign + '/prepare', ready['command'], expected=400,
            code='invalid_hearing_derived_deadline')


def report(mode, records):
    print('Hearing derived deadline evidence: ' + json.dumps({
        'mode': mode, 'origins': [record['origin'] for record in records],
        'calculations': [record['deadline']['calculation']['result'] for record in records],
    }, sort_keys=True))


def capture():
    scenario = fixture()
    saved = {'scenario': scenario, 'drafts': [], 'records': [], 'next_recovery_index': 1}
    save(saved)
    for blocked, token, actor in [
        (False, TOKEN, scenario['actor']),
        (True, scenario['accounts']['litigator']['token'],
         {key: scenario['accounts']['litigator'][key] for key in ['id', 'email', 'role']}),
    ]:
        value = command(scenario, blocked)
        ready = prepare(value, token)
        assert ready['state'] == 'ready' and ready['command'] == value
        assert prepare(value, token) == ready
        for field in ['recorded_at', 'source_event', 'capture_digest', 'receipt']:
            assert field not in ready['result'] and field not in ready['deadline']
        evaluation = ready['deadline']['result']
        assert bool(evaluation['blocks']) == blocked
        assert (evaluation['due_at'] is None) == blocked
        if not blocked:
            assert evaluation['due_at'] == {
                'unix_seconds': 1767817807, 'nanosecond': 0, 'offset_seconds': 0}
        absent(value)
        submit({**ready, 'review_digest': '00' * 32}, token, 409, 'deadline_submission_mismatch')
        altered = {**ready, 'command': changed(value)}
        submit(altered, token, 409, 'deadline_submission_mismatch')
        absent(value)
        counts(scenario, saved['records'])
        rejected_access(scenario, ready)
        record = submit(ready, token)
        assert_creation(ready, record, actor)
        saved['drafts'].append(ready)
        saved['records'].append(record)
        save(saved)
        exact_pair(record)
        # Treat the completed submit as a lost response, then reconcile its exact instruction.
        assert prepare(value, token) == {'state': 'replay', 'record': record}
        assert submit(ready, token) == record
        prepare(changed(value), token, 409, 'deadline_submission_mismatch')
        changed_operation = copy.deepcopy(value)
        changed_operation['deadline']['operation_id'] = str(uuid4())
        prepare(changed_operation, token, 409, 'deadline_operation_conflict')
        counts(scenario, saved['records'])
    member = route(scenario['case_id']) + '/members/' + scenario['accounts']['litigator']['id']
    request('DELETE', member, expected=204)
    prepare(saved['drafts'][1]['command'], scenario['accounts']['litigator']['token'], 404, 'case_not_found')
    submit(saved['drafts'][1], scenario['accounts']['litigator']['token'], 404, 'case_not_found')
    request('PUT', member, expected=204)
    assert submit(saved['drafts'][1], scenario['accounts']['litigator']['token']) == saved['records'][1]
    agenda(scenario, saved['records'])
    assert request('GET', '/api/v1/audit/verify')['valid']
    save(saved)
    report('capture', saved['records'])
    print('Compound API capture passed: calculable and blocked R1 pairs, reviewed origin, '
          'rejected changes/access, exact Replay and retry without duplicate roots.')


def reconcile(mode):
    saved = json.loads(STATE.read_text(encoding='ascii'))
    scenario = saved['scenario']
    assert request('GET', '/api/v1/auth/me')['id'] == scenario['actor']['id']
    assert len(saved['records']) == len(saved['drafts']) == 2
    account = scenario['accounts']['litigator']
    if mode == 'restore':
        request('GET', '/api/v1/auth/me', expected=401, token=account['token'], code='invalid_session')
    account['token'] = login(account, saved['next_recovery_index'])
    saved['next_recovery_index'] += 1
    save(saved)
    assert request('GET', '/api/v1/auth/me', token=account['token'])['id'] == account['id']
    for ready, record, token in zip(saved['drafts'], saved['records'], [TOKEN, account['token']]):
        exact_pair(record)
        assert prepare(ready['command'], token) == {'state': 'replay', 'record': record}
        assert submit(ready, token) == record
        assert_creation(ready, record, record['origin']['recorded_by'])
    counts(scenario, saved['records'])
    agenda(scenario, saved['records'])
    assert request('GET', '/api/v1/audit/verify')['valid']
    report(mode, saved['records'])
    print('Compound API reconciliation passed: original R1 evidence, authors, event and receipts '
          'survive restart/restore with authenticated Replay and no duplicate roots.')


if __name__ == '__main__':
    assert len(sys.argv) == 2 and sys.argv[1] in ['capture', 'verify', 'restore']
    capture() if sys.argv[1] == 'capture' else reconcile(sys.argv[1])
