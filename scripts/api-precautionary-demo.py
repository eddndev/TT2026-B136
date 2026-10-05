#!/usr/bin/env python3
"""Accept original precautionary operations through native restart and database restore."""
import copy
import json
import sys
import time
from uuid import uuid4

from api_precautionary_fixtures import (
    STATE, TOKEN, administrative, at, collection, context, decision, fixture, hearing,
    login, operation_path, replay, request, review, route, save, transact,
)
from api_precautionary_observation import (
    agenda, await_alert, exact_hearing_path, exact_measure_path, mark_read,
    paged_records, permissions, upcoming, verify_alerts,
)


def remember(saved, path, expected=None):
    value = request('GET', path)
    if expected is not None:
        assert value == expected, 'The exact capture changed: ' + path
    saved['records'][path] = value
    return value


def measure(saved, identifier):
    scenario = saved['scenario']
    value = request('GET', route(scenario['case_id']) + '/measures/' + identifier)
    remember(saved, exact_measure_path(scenario, value), value)
    return value


def write(saved, kind, command, role='owner', reject_digests=False):
    item = transact(saved['scenario'], kind, command, role, reject_digests)
    saved['operations'].append(item)
    remember(saved, operation_path(saved['scenario'], kind, command), item['operation'])
    if kind == 'hearing':
        remember(saved, exact_hearing_path(saved['scenario'], item['operation']), item['operation'])
    save(saved)
    return item['operation']


def report(mode, saved):
    print('Precautionary evidence: ' + json.dumps({
        'mode': mode, 'case_id': saved['scenario']['case_id'],
        'operations': [{'kind': item['kind'],
                        'operation_id': review(item['prepared'], item['kind'])['command']['operation_id'],
                        'submission_digest': review(item['prepared'], item['kind'])['submission_digest'],
                        'review_digest': review(item['prepared'], item['kind'])['review_digest']}
                       for item in saved['operations']],
        'measures': saved['measure_references'],
        'read_alert_id': saved['alerts']['read']['alert']['id'],
        'read_occurrence_id': saved['alerts']['read']['alert']['occurrence_id'],
        'email_transport': 'disabled',
    }, sort_keys=True))


def capture():
    scenario = fixture()
    saved = {'scenario': scenario, 'operations': [], 'records': {}, 'next_recovery_index': 1}
    save(saved)
    first = write(saved, 'hearing', hearing(scenario), reject_digests=True)
    account = scenario['accounts']['litigator']
    read = mark_read(first, account)
    initial = decision(scenario, first)
    declared = write(saved, 'decision', initial, 'litigator', True)
    assert declared['group']['decision']['anchor']['kind'] == 'precautionary'
    original = measure(saved, initial['outcome']['effects'][0]['proposal']['id'])
    assert original['record']['capture']['result']['values'] == initial['outcome']['effects'][0]['proposal']['values']
    assert original['record']['capture']['result']['sources']['subject'] == scenario['people'][0]['subject']
    correction = administrative(scenario, original)
    write(saved, 'administrative', correction, 'litigator', True)
    corrected = measure(saved, original['reference']['id'])
    assert corrected['reference']['revision'] == original['reference']['revision'] + 1
    assert corrected['family'] == 'c1' and corrected['validity'] == 'valid'
    assert corrected['record']['capture']['result']['values']['conditions'] == correction['action']['values']['conditions']
    assert corrected['judicial_origin'] == original['judicial_origin']
    replacement_command = administrative(scenario, corrected, replacement=True)
    replacement = write(saved, 'administrative', replacement_command, 'litigator')
    marked = measure(saved, original['reference']['id'])
    successor = measure(saved, replacement_command['action']['replacement_id'])
    assert marked['validity'] == 'entered_in_error'
    assert marked['reference']['revision'] == corrected['reference']['revision'] + 1
    assert successor['validity'] == 'valid' and successor['reference']['revision'] == 1
    assert successor['judicial_origin'] == original['judicial_origin']
    assert successor['record_root'] == {
        'kind': 'administrative', 'operation_id': replacement_command['operation_id'],
        'measure_id': successor['reference']['id']}
    assert successor['record']['capture']['result']['sources']['subject'] == scenario['people'][1]['subject']
    assert len(replacement['capture']['records']) == 2
    assert replacement['capture']['replacement_link'] == {
        'entered_in_error': marked['reference'], 'replacement': successor['reference']}
    saved['measure_references'] = [row['reference'] for row in [original, corrected, marked, successor]]

    second = write(saved, 'hearing', hearing(scenario, successor), 'litigator')
    assert second['capture']['review']['resolved_values']['review_targets'] == [successor['reference']]
    owners = second['history']['record_history']['records']['administrative']
    assert any(owner['capture'] == replacement['capture'] for owner in owners)
    second_alert = upcoming(second, account)
    second_review = second['capture']['review']
    replacement_values = copy.deepcopy(second_review['resolved_values'])
    replacement_values['scheduled_at'] = at(72)
    rescheduled = write(saved, 'hearing', {
        'case_id': scenario['case_id'], 'operation_id': str(uuid4()),
        'hearing_id': second_review['command']['hearing_id'],
        'change': {'action': 'replace', 'expected_revision': second_review['result_revision'],
                   'expected_capture_digest': second['capture']['capture_digest'],
                   'context': context(scenario), 'values': replacement_values,
                   'reason': 'Express rescheduling declaration'}}, 'litigator')
    resolved = await_alert(second, account['token'], resolved=True)
    assert resolved['id'] == second_alert['id']
    assert resolved['occurrence_id'] == second_alert['occurrence_id']
    assert resolved['origin'] == second_alert['origin']
    cancelled = write(saved, 'hearing', {
        'case_id': scenario['case_id'], 'operation_id': str(uuid4()),
        'hearing_id': second_review['command']['hearing_id'],
        'change': {'action': 'cancel', 'expected_revision': 2,
                   'expected_capture_digest': rescheduled['capture']['capture_digest'],
                   'reason': 'Express cancellation declaration'}}, 'litigator')
    assert cancelled['capture']['review']['status'] == 'cancelled'
    assert cancelled['capture']['review']['resolved_values'] == rescheduled['capture']['review']['resolved_values']
    assert cancelled['history']['captures'] == [second['capture'], rescheduled['capture'], cancelled['capture']]
    saved['hearings'] = [first, cancelled]
    saved['alerts'] = {'read': read, 'resolved': resolved}
    saved['agenda'] = agenda(scenario, saved['hearings'])
    verify_alerts(scenario, saved['hearings'], saved['alerts'])
    selected_paths = [exact_hearing_path(scenario, second),
                      operation_path(scenario, 'decision', initial),
                      operation_path(scenario, 'administrative', replacement_command),
                      exact_measure_path(scenario, original)]
    permissions(scenario, [saved['operations'][index] for index in [0, 1, 2]],
                selected_paths, read['alert'])
    for item in saved['operations']:
        replay(scenario, item)
    for path, expected in saved['records'].items():
        assert request('GET', path) == expected, 'A later head replaced historical evidence: ' + path
    paged_records(scenario, saved['records'])
    for person in scenario['people']:
        for family, row in [('subjects', person['subject']), ('participants', person['participant'])]:
            remember(saved, route(scenario['case_id']) + '/' + family + '/' + row['id'] + '/revisions/' + str(row['revision']))
    remember(saved, route(scenario['case_id']) + '/documents/' + scenario['support']['document_id'] + '/versions/1')
    assert request('GET', '/api/v1/audit/verify')['valid']
    save(saved)
    report('capture', saved)
    print('Precautionary capture passed: exact appointments, decision, correction and atomic replacement, '
          'historical reads, both confirmations, replay, staff access, agenda and native alerts.')


def reconcile(mode):
    saved = json.loads(STATE.read_text(encoding='ascii'))
    scenario = saved['scenario']
    assert request('GET', '/api/v1/auth/me')['id'] == scenario['owner']['id']
    index = saved['next_recovery_index']
    for account in scenario['accounts'].values():
        if mode == 'restore':
            request('GET', '/api/v1/auth/me', expected=401, token=account['token'], code='invalid_session')
        account['token'] = login(account, index)
        assert request('GET', '/api/v1/auth/me', token=account['token'])['id'] == account['id']
    saved['next_recovery_index'] += 1
    save(saved)
    token = scenario['accounts']['litigator']['token']
    assert request('GET', '/api/v1/alert-preferences', token=token)['preferences'] == scenario['preferences']
    for path, expected in saved['records'].items():
        assert request('GET', path) == expected, 'Restart or restore changed exact evidence: ' + path
    for item in saved['operations']:
        replay(scenario, item)
    pages = {}
    paged_records(scenario, pages)
    for path, expected in pages.items():
        assert expected == saved['records'][path]
    assert agenda(scenario, saved['hearings']) == saved['agenda']
    for _ in range(3):
        verify_alerts(scenario, saved['hearings'], saved['alerts'])
        time.sleep(1)
    selected = [exact_hearing_path(scenario, saved['hearings'][1]),
                operation_path(scenario, 'decision', review(saved['operations'][1]['prepared'], 'decision')['command'])]
    permissions(scenario, [], selected, saved['alerts']['read']['alert'])
    assert request('GET', '/api/v1/audit/verify')['valid']
    report(mode, saved)
    print('Precautionary reconciliation passed: original captures, operation identities, measure links, '
          'historical sources, agenda and read occurrence survived without duplicate writes.')


if __name__ == '__main__':
    assert len(sys.argv) == 2 and sys.argv[1] in ['capture', 'verify', 'restore']
    capture() if sys.argv[1] == 'capture' else reconcile(sys.argv[1])
