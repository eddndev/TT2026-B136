#!/usr/bin/env python3
"""Accept exact hearing ends over disposable HTTP and reconcile stored evidence."""
import json
import sys
from urllib.parse import urlencode

from api_hearing_anchor_fixtures import DUE, VARIANTS, command, expected_result, fixture
from api_hearing_derived_deadline_contract import assert_creation
from api_hearing_derived_deadline_fixtures import WORK, prepare, request, route, submit

STATE = WORK / 'hearing-anchor-api-state.json'


def save(value):
    STATE.write_text(json.dumps(value, sort_keys=True, ensure_ascii=True), encoding='ascii')
    STATE.chmod(0o600)


def result_path(value):
    return route(value['case_id']) + '/hearings/' + value['result']['hearing_id'] + '/results'


def absent(value):
    request('GET', result_path(value) + '/' + value['result']['result_id'] + '/revisions/1',
            expected=404, code='hearing_result_not_found')
    request('GET', route(value['case_id']) + '/deadlines/' + value['deadline']['deadline_id'],
            expected=404, code='deadline_not_found')


def exact_pair(record):
    value = record['command']
    assert request('GET', result_path(value) + '/' + record['result']['id'] + '/revisions/1') == record['result']
    assert request('GET', route(value['case_id']) + '/deadlines/'
                   + record['deadline']['id'] + '/revisions/1') == record['deadline']


def counts(saved):
    collection = route(saved['scenario']['case_id'])
    records = saved['records']
    page = request('GET', collection + '/deadlines?limit=100')
    assert not page['has_more'] and page['next_after_id'] is None
    assert len(page['deadlines']) == len(records)
    assert {row['id'] for row in page['deadlines']} == {row['deadline']['id'] for row in records}
    for record in records:
        page = request('GET', result_path(record['command']) + '?limit=100')
        assert not page['has_more'] and page['next_after_id'] is None
        assert len(page['results']) == 1
        assert page['results'][0]['id'] == record['result']['id']
        assert page['results'][0]['revision'] == 1
    hearings = request('GET', collection + '/hearings?limit=100')
    assert not hearings['has_more'] and hearings['next_after_id'] is None
    assert len(hearings['hearings']) == len(records)
    assert {row['id'] for row in hearings['hearings']} == {
        record['result']['hearing_id'] for record in records}


def agenda(saved):
    query = urlencode({'from': '2026-01-06T21:45:12Z', 'until': '2026-01-06T21:45:13Z',
                       'kind': 'deadline', 'limit': 100})
    page = request('GET', '/api/v1/agenda?' + query)
    assert page['complete'] and page['next_cursor'] is None
    rows = [row for row in page['items'] if row['kind'] == 'deadline'
            and row['deadline']['case_id'] == saved['scenario']['case_id']]
    assert len(rows) == 1, 'Only the exact declared end has an operational due instant'
    row = rows[0]
    assert row['deadline']['id'] == saved['records'][0]['deadline']['id']
    assert row['at'] == row['deadline']['operational']['due_at'] == DUE
    assert row['deadline']['operational']['freshness'] == 'current'
    assert row['deadline']['operational']['checked_at'] == page['checked_at']
    for record in saved['records'][1:]:
        value = request('GET', route(saved['scenario']['case_id']) + '/deadlines/' + record['deadline']['id'])
        assert value['calculation']['result']['due_at'] is None
        assert value['operational']['due_at'] is None


def profile_exact(scenario):
    profile = scenario['profile']
    assert request('GET', route(scenario['case_id']) + '/deadline-profiles/'
                   + profile['id'] + '/revisions/1') == profile


def report(mode, saved):
    print('Hearing anchor evidence: ' + json.dumps({
        'mode': mode, 'case_id': saved['scenario']['case_id'],
        'variants': VARIANTS, 'origins': [row['origin'] for row in saved['records']],
        'calculations': [row['deadline']['calculation']['result'] for row in saved['records']],
    }, sort_keys=True))


def capture():
    assert not STATE.exists(), 'Use verify or restore to reconcile an existing anchor capture'
    saved = {'scenario': fixture(), 'drafts': [], 'records': []}
    save(saved)
    scenario = saved['scenario']
    profile_exact(scenario)
    for variant in VARIANTS:
        value = command(scenario, variant)
        ready = prepare(value)
        assert ready['state'] == 'ready' and ready['command'] == value
        assert prepare(value) == ready
        assert ready['deadline']['result'] == expected_result(variant)
        absent(value)
        submit({**ready, 'review_digest': '00' * 32}, expected=409, code='deadline_submission_mismatch')
        absent(value)
        record = submit(ready)
        assert_creation(ready, record, scenario['actor'])
        assert record['deadline']['calculation']['result'] == expected_result(variant)
        saved['drafts'].append(ready)
        saved['records'].append(record)
        save(saved)
        exact_pair(record)
        assert prepare(value) == {'state': 'replay', 'record': record}
        assert submit(ready) == record
        counts(saved)
    agenda(saved)
    assert request('GET', '/api/v1/audit/verify')['valid']
    report('capture', saved)
    print('Hearing anchor capture passed: exact end, two explicit blocks, original source, '
          'reviewed origin, UTC agenda and retries without extra results, deadlines or hearings.')


def reconcile(mode):
    saved = json.loads(STATE.read_text(encoding='ascii'))
    scenario = saved['scenario']
    assert request('GET', '/api/v1/auth/me')['id'] == scenario['actor']['id']
    assert len(saved['records']) == len(saved['drafts']) == len(VARIANTS)
    profile_exact(scenario)
    for variant, ready, record in zip(VARIANTS, saved['drafts'], saved['records']):
        assert_creation(ready, record, scenario['actor'])
        assert record['deadline']['calculation']['result'] == expected_result(variant)
        exact_pair(record)
        assert prepare(ready['command']) == {'state': 'replay', 'record': record}
        assert submit(ready) == record
    counts(saved)
    agenda(saved)
    assert request('GET', '/api/v1/audit/verify')['valid']
    report(mode, saved)
    print('Hearing anchor reconciliation passed: original profile, sources, results, deadlines, '
          'origins and receipts survive with exact Replay and no duplicate roots.')


if __name__ == '__main__':
    assert len(sys.argv) == 2 and sys.argv[1] in ['capture', 'verify', 'restore']
    capture() if sys.argv[1] == 'capture' else reconcile(sys.argv[1])
