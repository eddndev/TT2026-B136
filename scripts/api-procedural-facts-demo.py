#!/usr/bin/env python3
"""Verify declared resolutions and notifications over HTTP and after restoration."""
import copy
import json
import sys
from uuid import uuid4
from api_procedural_facts_support import (
    STATE, TOKEN, base, change, enroll, evidence, exact_paths, external,
    new_case, prepare, receipt, record, request, submit, upload, values,
)
from api_procedural_facts_history import (
    authorization_and_closure, historical_baseline, historical_result,
)


def capture():
    route = new_case('MAIN')
    other = new_case('FOREIGN')
    users, tokens = {}, {}
    for role in ['litigator', 'paralegal', 'client']:
        users[role], tokens[role] = enroll(role)
        request('PUT', route + '/members/' + users[role], expected=204)
    parent_support = upload(route, 'pdf', 'fact-parent.pdf')
    receipt_support = upload(route, 'pdf', 'fact-receipt.pdf')
    representation_support = upload(route, 'docx', 'fact-representation.docx')
    assert parent_support['digest'] == receipt_support['digest']
    assert parent_support['document_id'] != receipt_support['document_id']
    value = values('resolution')
    value['summary'] = ' Original declared resolution\r\nPreserved history '
    value['issued_at'] = {'precision': 'minute', 'year': 2026, 'month': 9, 'day': 1,
                          'hour': 10, 'minute': 15, 'offset_seconds': None}
    value['provenance'] = external(parent_support, 'Parent page 1')
    command = record('resolution', value)
    command['id'] = '00000000-0000-0000-0000-000000000000'
    root_list = route + '/resolutions'
    assert request('GET', root_list)['resolutions'] == []
    draft = prepare(route, command)
    assert prepare(route, command) == draft
    assert request('GET', root_list)['resolutions'] == []
    assert len(draft['sources']['direct_supports']) == 1
    assert draft['observed_administration']['kind'] == 'recorded'
    assert 'profile' not in draft['observed_administration']
    submit(route, {**draft, 'submission_digest': '00' * 32}, expected=409,
           code='procedural_fact_submission_mismatch')
    first = submit(route, draft)
    receipt(first, draft)
    parent_path = root_list + '/' + first['id']
    assert first['values']['summary'] == 'Original declared resolution\nPreserved history'
    assert first['values']['issued_at']['offset_seconds'] is None
    assert 'second' not in first['values']['issued_at']
    assert request('GET', parent_path + '/revisions/1') == first
    submit(route, draft, expected=409)
    request('GET', other + '/resolutions/' + first['id'], expected=404)
    request('GET', parent_path + '/revisions/99', expected=404, code='procedural_fact_not_found')

    # A later document version never replaces the exact admitted parent version.
    request('POST', route + '/documents/' + parent_support['document_id'] + '/versions?expected_version=1',
            b'Later unrelated bytes', 201, headers={'X-Document-Name': 'later-parent.txt'})
    correction = change(first, 'correct')
    correction['change']['values']['summary'] = 'Corrected declared resolution'
    stale = prepare(route, correction)
    winner = copy.deepcopy(correction)
    winner['operation_id'] = str(uuid4())
    second = submit(route, prepare(route, winner))
    assert second['revision'] == 2 and second['sources'] == first['sources']
    submit(route, stale, expected=409, code='procedural_fact_revision_conflict')
    withdrawal = prepare(route, change(second, 'withdraw'))
    parent = submit(route, withdrawal)
    receipt(parent, withdrawal)
    assert parent['revision'] == 3 and parent['status'] == 'withdrawn'
    assert parent['values'] == second['values'] and parent['sources'] == second['sources']
    prepare(route, change(parent, 'correct'), expected=409, code='procedural_fact_already_withdrawn')

    person = request('POST', route + '/participants', {
        'display_name': 'Historical recipient', 'procedural_role': 'Declared recipient',
    }, 201)
    participant_path = route + '/participants/' + person['id']
    request('PUT', participant_path + '/directory-status', {
        'expected_revision': 1, 'directory_status': 'archived',
    })
    declared = {'kind': 'participant', 'id': person['id'], 'revision': 2}
    value = values('notification')
    value['resolution'] = {'id': parent['id'], 'revision': 3}
    value['summary'] = 'Declared practice with two immediate supports'
    value['practiced_at'] = {'precision': 'date', 'year': 2026, 'month': 9, 'day': 2, 'offset_seconds': 0}
    value['received_at'] = {'precision': 'unknown'}
    value['intended_recipient'] = {'kind': 'known', 'value': declared}
    value['provenance'] = external(receipt_support, 'Receipt page 1')
    value['representation'] = {
        'kind': 'declared', 'represented': declared,
        'representative': {'kind': 'unlinked', 'label': 'Declared representative',
                           'description': 'Reported person without a directory identity'},
        'scope': 'Declared limited representation',
        'provenance': external(representation_support, 'Representation paragraph 1'),
    }
    command = record('notification', value, parent['id'])
    command['id'] = parent['id']
    notice_list = base(route, command)
    assert request('GET', notice_list)['notifications'] == []
    draft = prepare(route, command, tokens['litigator'])
    direct = draft['sources']['direct_supports']
    assert len(direct) == 2 and {row['format'] for row in direct} == {'pdf', 'docx'}
    assert {row['document_id'] for row in direct} == {
        receipt_support['document_id'], representation_support['document_id'],
    }
    assert parent_support['document_id'] not in {row['document_id'] for row in direct}
    assert draft['sources']['resolution']['status'] == 'withdrawn'
    assert draft['sources']['resolution']['revision'] == 3
    assert draft['sources']['participants'][0]['directory_status'] == 'archived'
    notice_first = submit(route, draft, tokens['litigator'])
    receipt(notice_first, draft)
    notice_path = notice_list + '/' + notice_first['id']
    assert request('GET', notice_path + '/revisions/1') == notice_first
    request('PUT', participant_path + '/directory-status', {
        'expected_revision': 2, 'directory_status': 'active',
    })
    assert request('GET', notice_path + '/revisions/1') == notice_first
    replacement = copy.deepcopy(notice_first['values'])
    replacement['resolution']['revision'] = 1
    replacement['summary'] = 'Correction selects another exact revision of the same parent'
    command = change(notice_first, 'correct', replacement)
    stale_notice = prepare(route, command)
    competing = copy.deepcopy(command)
    competing['operation_id'] = str(uuid4())
    notice_second = submit(route, prepare(route, competing))
    assert notice_second['sources']['resolution']['status'] == 'recorded'
    assert notice_second['sources']['resolution']['revision'] == 1
    assert notice_second['sources']['resolution']['summary'] == first['values']['summary']
    assert notice_second['sources']['participants'] == notice_first['sources']['participants']
    submit(route, stale_notice, expected=409, code='procedural_fact_revision_conflict')
    notice = submit(route, prepare(route, change(notice_second, 'withdraw')))
    assert notice['status'] == 'withdrawn' and notice['revision'] == 3
    assert notice['values'] == notice_second['values'] and notice['sources'] == notice_second['sources']
    prepare(route, change(notice, 'correct'), expected=409, code='procedural_fact_already_withdrawn')

    # Referencing a parent with a support does not add it to an empty direct lot.
    minimal = values('notification')
    minimal['resolution'] = {'id': parent['id'], 'revision': 3}
    no_support = submit(route, prepare(route, record('notification', minimal, parent['id'])))
    assert no_support['sources']['direct_supports'] == []
    assert no_support['values']['received_at'] is None
    assert notice_first['values']['received_at'] == {'precision': 'unknown'}
    invalid = copy.deepcopy(minimal)
    invalid['resolution']['revision'] = 99
    prepare(route, record('notification', invalid, parent['id']), expected=404,
            code='procedural_fact_reference_not_found')
    foreign = request('POST', other + '/participants', {
        'display_name': 'Foreign source', 'procedural_role': 'Witness',
    }, 201)
    invalid['resolution']['revision'] = 3
    invalid['intended_recipient'] = {'kind': 'known', 'value': {
        'kind': 'participant', 'id': foreign['id'], 'revision': 1,
    }}
    prepare(route, record('notification', invalid, parent['id']), expected=404,
            code='procedural_fact_reference_not_found')
    reused = record('resolution', values('resolution'))
    reused['operation_id'] = first['receipt']['operation_id']
    prepare(route, reused, expected=409, code='procedural_fact_operation_conflict')
    for path in [parent_path + '?revision=1', notice_path + '/revisions/1?extra=1',
                 root_list + '?limit=1&limit=2']:
        request('GET', path, expected=400, code='invalid_query')
    history = request('GET', notice_path + '/history?limit=2')
    assert [row['revision'] for row in history['revisions']] == [3, 2] and history['has_more']
    assert all('values' not in row and 'sources' not in row for row in history['revisions'])
    assert request('GET', notice_path + '/history?before_revision=2')['revisions'][0]['revision'] == 1
    page = request('GET', notice_list + '?limit=1&status=all')
    assert len(page['notifications']) == 1 and page['has_more']
    tail = request('GET', notice_list + '?status=all&after_id=' + page['next_after_id'])
    assert len(tail['notifications']) == 1 and not tail['has_more']
    assert len(request('GET', notice_list + '?status=withdrawn')['notifications']) == 1
    assert len(request('GET', notice_list + '?status=recorded')['notifications']) == 1
    paths = authorization_and_closure(route, other, tokens, users, parent, notice)
    paths += exact_paths(route, parent) + exact_paths(route, notice) + exact_paths(route, no_support)
    paths += [root_list + '?status=all', notice_list + '?status=all', notice_list + '?status=withdrawn',
              notice_path + '/history?limit=2', participant_path + '/revisions/2']
    paths += historical_result() + historical_baseline()
    records = {path: request('GET', path) for path in dict.fromkeys(paths)}
    STATE.write_text(json.dumps({'records': records}, sort_keys=True), encoding='utf-8')
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Fact API passed: two families, exact receipts, four roles, historical sources, 0..2 direct supports, conflicts and closure.')
    print('Fact restore capture: ' + str(len(records)) + ' exact HTTP responses.')


def restore():
    records = json.loads(STATE.read_text(encoding='utf-8'))['records']
    for path, expected in records.items():
        assert request('GET', path) == expected, 'Restored fact response differs: ' + path
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Fact restore passed: ' + str(len(records)) + ' exact responses, historical sources and receipts.')


if __name__ == '__main__':
    {'capture': capture, 'restore': restore}[sys.argv[1]]()
