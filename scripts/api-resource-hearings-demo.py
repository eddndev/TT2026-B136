#!/usr/bin/env python3
"""Accept own hearing creation, exact replay, live alerts and full database restore."""
import copy
import json
import sys
import time
from uuid import uuid4

from api_resource_hearings_support import (
    STATE, TOKEN, absent, activities, base, command, creation, exact, fixture,
    listing, login, prepare, request, resources, route, save, submit,
)
from api_resource_hearings_observation import (
    agenda, denied_alerts, own_alerts, read, upcoming, verify_read,
)
from api_resource_hearings_access import conflict, permissions, retained_history


def report(mode, results, readings):
    records = []
    for result in results:
        row = result['hearing']
        records.append({key: row[key] for key in [
            'case_id', 'resource_id', 'id', 'revision', 'association_id',
            'operation_id', 'resource', 'act', 'recorded_resource_head',
            'submission_digest', 'capture_digest',
        ]})
    print('Resource hearing evidence: ' + json.dumps({
        'mode': mode, 'hearings': records,
        'alert_id': readings['owner']['alert']['id'],
        'occurrence_id': readings['owner']['alert']['occurrence_id'],
        'email_transport': 'disabled',
    }, sort_keys=True))


def capture():
    scenario = fixture()
    collection = base(scenario['resource'])
    links = activities.base(scenario['resource'])
    first_command = command(scenario)
    assert scenario['resource']['revision'] == 1 and scenario['act']['revision'] == 2
    assert scenario['head']['revision'] == 3 and scenario['head']['act']['revision'] == 2
    first_draft = prepare(collection, first_command)
    assert first_draft['command'] == first_command
    assert prepare(collection, first_command) == first_draft
    assert request('GET', collection)['items'] == []
    assert request('GET', links)['associations'] == []
    submit(collection, {**first_draft, 'submission_digest': '00' * 32}, expected=409,
           code='resource_activity_submission_mismatch')
    absent(collection, first_command)
    first = submit(collection, first_draft)
    creation(first, first_draft, scenario)
    assert submit(collection, first_draft) == first
    assert prepare(collection, first_command) == first_draft
    assert request('GET', exact(collection, first)) == first
    request('GET', collection + '/' + first['hearing']['id'] + '/revisions/2', expected=404,
            code='resource_activity_not_found')
    changed = copy.deepcopy(first_command)
    changed['association_id'] = str(uuid4())
    prepare(collection, changed, expected=409, code='resource_activity_operation_conflict')
    other_parent = collection.replace(scenario['resource']['id'], str(uuid4()))
    request('GET', other_parent + '/' + first['hearing']['id'], expected=404,
            code='resource_activity_not_found')

    token = scenario['accounts']['litigator']['token']
    second_command = command(scenario, hours=72, with_act=False)
    second_draft = prepare(collection, second_command, token)
    second = submit(collection, second_draft, token)
    creation(second, second_draft, scenario)
    assert second['hearing']['recorded_by']['id'] == scenario['accounts']['litigator']['id']
    assert second['hearing']['act'] is None and second['hearing']['sources']['act'] is None
    results, drafts = [first, second], [first_draft, second_draft]
    listing(collection, results)
    assert len(request('GET', links)['associations']) == 2
    agenda(results)

    readings = {'owner': read(first, scenario['owner']['id'])}
    staff_alerts = {}
    for role in ['litigator', 'paralegal']:
        account = scenario['accounts'][role]
        staff_alerts[role] = upcoming(first, account['id'], account['token'])
    assert own_alerts(second) == [], 'The later hearing has not entered its alert window'
    denied_alerts(scenario, first, readings['owner']['alert'])
    conflict(scenario, results)
    permissions(scenario, results, drafts, staff_alerts['litigator'])
    removed = retained_history(scenario, results, drafts)
    expected_agenda = agenda(results, status='closed')
    verify_read(first, readings['owner'])
    for role in ['litigator', 'paralegal']:
        account = scenario['accounts'][role]
        assert upcoming(first, account['id'], account['token']) == staff_alerts[role]
    paths = [collection, *[exact(collection, item) for item in results], links,
             links + '/' + removed['id'], links + '/' + removed['id'] + '/history',
             links + '/' + removed['id'] + '/revisions/1',
             route(scenario['case_id']) + '/participants/' + scenario['participant']['id'] + '/revisions/1',
             *resources.exact_paths(route(scenario['case_id']), scenario['head'])]
    records = {path: activities.stable(request('GET', path)) for path in paths}
    save({'case_id': scenario['case_id'], 'scenario': scenario, 'drafts': drafts,
          'results': results, 'records': records, 'agenda': expected_agenda,
          'readings': readings, 'staff_alerts': staff_alerts})
    assert request('GET', '/api/v1/audit/verify')['valid']
    report('capture', results, readings)
    print('Own hearing API passed: historical sources, atomic capture, exact replay, bounded list, '
          'agenda, live internal alerts, staff access and retained history after unlink/archive/closure.')


def restore():
    saved = json.loads(STATE.read_text(encoding='ascii'))
    scenario, results, drafts = saved['scenario'], saved['results'], saved['drafts']
    collection = base(scenario['resource'])
    assert request('GET', '/api/v1/auth/me')['id'] == scenario['owner']['id']
    for path, expected in saved['records'].items():
        assert activities.stable(request('GET', path)) == expected, 'Restored own hearing differs: ' + path
    for account in scenario['accounts'].values():
        request('GET', '/api/v1/auth/me', expected=401, token=account['token'], code='invalid_session')
        account['token'] = login(account, 1)
    assert request('GET', '/api/v1/alert-preferences',
                   token=scenario['accounts']['litigator']['token'])['preferences'] == scenario['preferences']
    for result, draft, token in zip(results, drafts, [TOKEN, scenario['accounts']['litigator']['token']]):
        assert submit(collection, draft, token) == result
        assert prepare(collection, draft['command'], token) == draft
        assert request('GET', exact(collection, result), token=token) == result
    listing(collection, results)
    assert agenda(results, status='closed') == saved['agenda']
    verify_read(results[0], saved['readings']['owner'])
    for role in ['litigator', 'paralegal']:
        account = scenario['accounts'][role]
        assert upcoming(results[0], account['id'], account['token']) == saved['staff_alerts'][role]
    denied_alerts(scenario, results[0], saved['readings']['owner']['alert'])
    # Observe again after several normal worker pauses, retaining the original occurrence.
    for _ in range(3):
        time.sleep(1)
        assert own_alerts(results[0]) == [saved['readings']['owner']['alert']]
    assert request('GET', '/api/v1/audit/verify')['valid']
    report('restore', results, saved['readings'])
    print('Own hearing restore passed: exact captures, original associations and authors, '
          'replayed commands, agenda and one read occurrence after worker restart.')


if __name__ == '__main__':
    assert len(sys.argv) == 2 and sys.argv[1] in ['capture', 'restore']
    {'capture': capture, 'restore': restore}[sys.argv[1]]()
