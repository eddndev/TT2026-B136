#!/usr/bin/env python3
"""Accept atomic contextual deadline creation and restore exact captured receipts."""
import copy
import json
import sys
from urllib.parse import urlencode
from uuid import uuid4
import api_procedural_facts_support as facts
import api_procedural_resources_support as resources
from api_deadlines_support import profile, register
from api_resource_activities_support import instant, stable

WORK, request = facts.WORK, facts.request
STATE = WORK / 'contextual-deadline-api-state.json'


def reference(row):
    return {'id': row['id'], 'revision': row['revision'],
            'capture_digest': row['receipt']['capture_digest']}


def prepare(base, command):
    return request('POST', base + '/deadlines/prepare', command)


def submit(base, draft, expected=201, code=None):
    return request('POST', base + '/deadlines/submit', {
        'command': draft['command'], 'expected_submission_digest': draft['submission_digest'],
    }, expected=expected, code=code)


def agenda(deadline):
    query = urlencode({'from': '2026-01-31T00:00:00Z', 'until': '2026-02-01T00:00:00Z',
                       'kind': 'deadline', 'limit': 100})
    page = request('GET', '/api/v1/agenda?' + query)
    instant(page['checked_at'])
    assert page['complete'] and page['next_cursor'] is None
    selected = [row for row in page['items'] if row['kind'] == 'deadline'
                and row['deadline']['id'] == deadline['id']]
    assert len(selected) == 1, 'Contextual deadline must appear exactly once in the combined agenda'
    row = selected[0]
    assert row['deadline']['case_id'] == deadline['case_id']
    assert row['deadline']['operational']['freshness'] == 'current'
    assert row['deadline']['operational']['checked_at'] == page['checked_at']
    assert row['at'] == row['deadline']['operational']['due_at'] == deadline['calculation']['result']['due_at']
    return stable(row)


def capture():
    actor = request('GET', '/api/v1/auth/me')
    case = request('POST', '/api/v1/cases', {
        'title': 'Contextual deadline acceptance', 'reference': 'CONTEXTUAL-DEADLINE-API',
    }, expected=201)
    route = '/api/v1/cases/' + case['id']
    support = facts.evidence(facts.upload(route, 'pdf', 'contextual-deadline-source.pdf'), 'Page 1')
    values = facts.values('resolution')
    values['issued_at'] = {'precision': 'date', 'year': 2026, 'month': 1, 'day': 31, 'offset_seconds': None}
    values['summary'] = 'Active source for a declared contextual deadline'
    values['provenance'] = facts.external(support, 'Page 1')
    source = facts.submit(route, facts.prepare(route, facts.record('resolution', values)))
    first = resources.submit(route, resources.prepare(route, resources.registration(source, support)))
    act_values = resources.act_values('written', support)
    act_values['occurred_at'] = copy.deepcopy(values['issued_at'])
    head = resources.submit(route, resources.prepare(route, resources.change(first, 'record_act', act_values)))
    base = route + '/procedural-resources/' + first['id'] + '/activities'
    selected_profile = profile(case['id'], 'daily')
    deadline_command = register(case['id'], selected_profile, source, actor['id'])
    deadline_command['change']['definition']['title'] = 'Deadline created with its resource association'
    command = {
        'case_id': case['id'], 'resource_id': first['id'], 'association_id': str(uuid4()),
        'expected_resource_revision': head['revision'], 'resource': reference(first),
        'act': {'id': head['act']['id'], 'revision': head['act']['revision'],
                'resource_revision': head['revision'], 'capture_digest': head['receipt']['capture_digest']},
        'deadline': deadline_command,
    }
    draft = prepare(base, command)
    assert draft['command'] == command and prepare(base, command) == draft
    assert draft['association']['resource'] == first and draft['association']['act'] == head
    assert draft['deadline']['calculation']['result']['blocks'] == []
    assert draft['deadline']['calculation']['result']['due_at'] is not None
    assert request('GET', route + '/deadlines')['deadlines'] == []
    assert request('GET', base)['associations'] == []
    submit(base, {**draft, 'submission_digest': '00' * 32}, 409,
           'resource_activity_submission_mismatch')
    assert request('GET', route + '/deadlines')['deadlines'] == []
    assert request('GET', base)['associations'] == []
    result = submit(base, draft)
    assert set(result) == {'deadline', 'association', 'submission_digest'}
    assert result['submission_digest'] == draft['submission_digest']
    deadline, association = result['deadline'], result['association']
    assert deadline['id'] == deadline_command['deadline_id'] and deadline['revision'] == 1
    assert association['id'] == command['association_id'] and association['revision'] == 1
    assert deadline['receipt']['operation_id'] == association['receipt']['operation_id'] == deadline_command['operation_id']
    assert deadline['receipt']['submission_digest'] == draft['deadline']['submission_digest']
    assert association['receipt']['submission_digest'] == draft['association']['submission_digest']
    assert association['selection']['resource'] == command['resource']
    assert association['selection']['act'] == command['act']
    assert association['selection']['target'] == {'kind': 'deadline', **reference(deadline)}
    assert association['sources']['resource'] == first and association['sources']['act'] == head
    assert association['sources']['target'] == {'kind': 'deadline', 'record': deadline}
    assert submit(base, draft) == result
    assert len(request('GET', route + '/deadlines')['deadlines']) == 1
    assert len(request('GET', base)['associations']) == 1
    expected_agenda = agenda(deadline)

    competing = copy.deepcopy(command)
    competing['association_id'] = str(uuid4())
    competing['deadline']['operation_id'] = str(uuid4())
    competing['deadline']['deadline_id'] = str(uuid4())
    stale = prepare(base, competing)
    replacement = copy.deepcopy(head['values'])
    replacement['title'] += ' corrected concurrently'
    current = resources.submit(route, resources.prepare(route, resources.change(head, 'correct', replacement)))
    submit(base, stale, 409, 'resource_activity_resource_revision_conflict')
    request('GET', route + '/deadlines/' + competing['deadline']['deadline_id'], expected=404,
            code='deadline_not_found')
    request('GET', base + '/' + competing['association_id'], expected=404,
            code='resource_activity_not_found')
    assert len(request('GET', route + '/deadlines')['deadlines']) == 1
    assert len(request('GET', base)['associations']) == 1
    assert agenda(deadline) == expected_agenda
    deadline_path, association_path = route + '/deadlines/' + deadline['id'], base + '/' + association['id']
    paths = [route + '/deadlines', base, deadline_path, deadline_path + '/revisions/1',
             deadline_path + '/history', association_path, association_path + '/revisions/1',
             association_path + '/history', *resources.exact_paths(route, current)]
    records = {path: stable(request('GET', path)) for path in paths}
    STATE.write_text(json.dumps({'records': records, 'base': base, 'draft': draft,
                                 'result': result, 'agenda': expected_agenda}, sort_keys=True))
    STATE.chmod(0o600)
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Contextual deadline API passed: atomic confirmation, exact replay, agenda and rejected stale preparation.')


def restore():
    saved = json.loads(STATE.read_text())
    for path, expected in saved['records'].items():
        assert stable(request('GET', path)) == expected, 'Restored contextual response differs: ' + path
    assert submit(saved['base'], saved['draft']) == saved['result']
    assert agenda(saved['result']['deadline']) == saved['agenda']
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Contextual deadline restore passed: exact deadline and association captures, receipts and agenda.')


if __name__ == '__main__':
    assert len(sys.argv) == 2 and sys.argv[1] in ['capture', 'restore']
    {'capture': capture, 'restore': restore}[sys.argv[1]]()
