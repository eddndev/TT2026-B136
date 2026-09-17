"""Reuse historical sources without altering prior demo captures or source heads."""
import csv
import json
from uuid import uuid4
from api_procedural_facts_support import (
    WORK, change, exact_paths, prepare, record, request, submit, values,
)


def historical_result():
    captured = json.loads((WORK / 'hearing-api-state.json').read_text())['records']
    for result in captured.values():
        if not isinstance(result, dict) or result.get('status') != 'withdrawn' or 'anchor' not in result:
            continue
        route = '/api/v1/cases/' + result['case_id']
        admin = request('GET', route + '/administration')['administration']
        if admin['administrative_status'] != 'active':
            continue
        value = values('resolution')
        agreement = result['values']['agreements'][0]
        value['provenance'] = {'kind': 'hearing_result', 'reference': {
            'hearing_id': result['hearing_id'], 'result_id': result['id'],
            'revision': result['revision'], 'agreement_id': agreement['id'],
        }, 'locator': 'Exact declared agreement', 'support': None}
        draft = prepare(route, record('resolution', value))
        assert draft['sources']['direct_supports'] == []
        source = draft['sources']['hearing_results'][0]
        assert source['status'] == 'withdrawn' and source['agreement'] == agreement
        assert source['submission_digest'] == result['receipt']['submission_digest']
        row = submit(route, draft)
        assert row['sources'] == draft['sources']
        return exact_paths(route, row)
    raise AssertionError('Expected an active case with a captured withdrawn hearing result')


def historical_baseline():
    with (WORK / 'case-baselines.csv').open(newline='') as source:
        baselines = list(csv.DictReader(source))
    for baseline in baselines:
        route = '/api/v1/cases/' + baseline['id']
        admin = request('GET', route + '/administration')['administration']
        if admin['revision'] != 0:
            continue
        draft = prepare(route, record('resolution', values('resolution')))
        observed = draft['observed_administration']
        assert observed['kind'] == 'unrevised'
        assert 'revision' not in observed and 'values_digest' not in observed
        row = submit(route, draft)
        assert row['recorded_administration'] == observed
        print('Fact baseline passed: imported unrevised metadata retained without an invented revision.')
        return exact_paths(route, row)
    print('Fact baseline omitted: every imported case already has a recorded administration.')
    return []


def authorization_and_closure(route, other, tokens, users, resolution, notification):
    resolution_path = route + '/resolutions/' + resolution['id']
    notice_base = resolution_path + '/notifications'
    for prefix in [route + '/resolutions', notice_base]:
        request('GET', prefix, token=tokens['paralegal'])
        request('GET', prefix, expected=403, token=tokens['client'])
    for row in [resolution, notification]:
        command = change(row, 'correct')
        prepare(route, command, tokens['paralegal'], 403, 'permission_denied')
        prepare(route, command, tokens['client'], 403, 'permission_denied')
    pending = prepare(route, record('resolution', values('resolution')), tokens['litigator'])
    request('DELETE', route + '/members/' + users['litigator'], expected=204)
    submit(route, pending, tokens['litigator'], 404, 'case_not_found')
    hidden = request('GET', other + '/resolutions', expected=404, token=tokens['litigator'])
    missing = request('GET', '/api/v1/cases/' + str(uuid4()) + '/resolutions',
                      expected=404, token=tokens['litigator'])
    assert hidden == missing
    request('PUT', route + '/members/' + users['litigator'], expected=204)
    admin = request('GET', route + '/administration')['administration']
    revision = admin['revision']
    request('PUT', route + '/administrative-status', {
        'expected_revision': revision, 'administrative_status': 'closed',
    })
    submit(route, pending, tokens['litigator'], 409, 'case_closed')
    for path in [resolution_path, notice_base + '/' + notification['id']]:
        request('GET', path, token=tokens['paralegal'])
    request('PUT', route + '/administrative-status', {
        'expected_revision': revision + 1, 'administrative_status': 'active',
    })
    confirmed = submit(route, pending, tokens['litigator'])
    assert confirmed['recorded_administration']['revision'] == revision + 2
    assert confirmed['receipt']['submission_digest'] == pending['submission_digest']
    return exact_paths(route, confirmed)
