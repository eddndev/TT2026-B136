#!/usr/bin/env python3
"""Accept declared resource history over HTTP and compare it after restoration."""
import copy
import json
import sys
from uuid import uuid4
from api_procedural_resources_support import (
    STATE, TOKEN, act_values, base, change, enroll, exact_paths, fixtures,
    prepare, receipt, registration, request, submit,
)


def authorization(route, head, users, tokens):
    path = base(route) + '/' + head['id']
    for token in [TOKEN, tokens['litigator'], tokens['paralegal']]:
        for suffix in ['', '/history', '/revisions/1']:
            request('GET', path + suffix, token=token)
    for suffix in ['', '/history', '/revisions/1']:
        request('GET', path + suffix, expected=403, token=tokens['client'], code='permission_denied')
    request('GET', base(route), expected=403, token=tokens['client'], code='permission_denied')
    command = change(head, 'correct')
    for role in ['paralegal', 'client']:
        prepare(route, command, tokens[role], 403, 'permission_denied')
    draft = prepare(route, command, tokens['litigator'])
    for role in ['paralegal', 'client']:
        submit(route, draft, tokens[role], 403, 'permission_denied')
    request('DELETE', route + '/members/' + users['litigator'], expected=204)
    submit(route, draft, tokens['litigator'], 404, 'case_not_found')
    hidden = request('GET', path, expected=404, token=tokens['litigator'])
    missing = request('GET', '/api/v1/cases/' + str(uuid4()) + '/procedural-resources/' + head['id'],
                      expected=404, token=tokens['litigator'])
    assert hidden == missing
    request('PUT', route + '/members/' + users['litigator'], expected=204)
    admin = request('GET', route + '/administration')['administration']
    request('PUT', route + '/administrative-status', {
        'expected_revision': admin['revision'], 'administrative_status': 'closed',
    })
    submit(route, draft, tokens['litigator'], 409, 'case_closed')
    assert request('GET', path, token=tokens['paralegal']) == head
    prepare(route, command, expected=409, code='case_closed')
    request('PUT', route + '/administrative-status', {
        'expected_revision': admin['revision'] + 1, 'administrative_status': 'active',
    })
    submit(route, draft, tokens['litigator'], 409, 'procedural_resource_submission_mismatch')
    renewed = prepare(route, command, tokens['litigator'])
    assert renewed['submission_digest'] != draft['submission_digest']
    result = submit(route, renewed, tokens['litigator'])
    receipt(result, renewed)
    assert result['recorded_administration']['revision'] == admin['revision'] + 2
    return result


def capture():
    route, parent, support, docx, participant = fixtures()
    collection = base(route)
    users, tokens = {}, {}
    for role in ['litigator', 'paralegal', 'client']:
        users[role], tokens[role] = enroll(role)
        request('PUT', route + '/members/' + users[role], expected=204)
    initial_facts = request('GET', route + '/resolutions/' + parent['id'] + '/history')
    initial_stage = request('GET', route + '/stage')
    assert request('GET', collection)['resources'] == []
    command = registration(parent, support)
    command['change']['values']['appellants'].append({
        'name': participant['display_name'],
        'role': {'kind': 'known', 'value': participant['procedural_role']},
        'participant': {'id': participant['id'], 'revision': participant['revision']},
    })
    draft = prepare(route, command)
    assert prepare(route, command) == draft
    assert request('GET', collection)['resources'] == []
    assert draft['sources']['resolution']['revision'] == 1
    assert draft['sources']['resolution']['summary'] == parent['values']['summary']
    assert draft['sources']['supports'][0]['version'] == 1
    assert draft['sources']['appellants'][0] == participant
    assert participant['directory_status'] == 'archived'
    submit(route, {**draft, 'submission_digest': '00' * 32}, expected=409,
           code='procedural_resource_submission_mismatch')
    first = submit(route, draft)
    receipt(first, draft)
    path = collection + '/' + first['id']
    assert first['act'] is None
    assert submit(route, draft) == first
    assert prepare(route, command) == draft
    assert len(request('GET', path + '/history')['revisions']) == 1
    replay_mismatch = copy.deepcopy(command)
    replay_mismatch['change']['values']['title'] = 'Different command under same operation'
    prepare(route, replay_mismatch, expected=409, code='procedural_resource_operation_conflict')
    reused = registration(parent, support, 'appeal')
    reused['operation_id'] = command['operation_id']
    prepare(route, reused, expected=409, code='procedural_resource_operation_conflict')

    appeal_draft = prepare(route, registration(parent, support, 'appeal'), tokens['litigator'])
    appeal_first = submit(route, appeal_draft, tokens['litigator'])
    receipt(appeal_first, appeal_draft)
    assert appeal_first['recorded_by']['id'] == users['litigator']
    oral = act_values('oral', docx)
    recorded = submit(route, prepare(route, change(first, 'record_act', oral)))
    assert recorded['revision'] == 2 and recorded['act']['revision'] == 1
    assert recorded['act']['values']['mode'] == {'kind': 'known', 'value': 'oral'}
    assert recorded['act']['values']['occurred_at']['offset_seconds'] is None
    assert 'hour' not in recorded['act']['values']['occurred_at']
    assert recorded['act']['supports'][0]['format'] == 'docx'
    assert recorded['sources'] == first['sources'] and recorded['values'] == first['values']
    corrected_values = copy.deepcopy(oral)
    corrected_values['statement'] = 'Correction of the declared oral act only'
    corrected = submit(route, prepare(route, change(recorded, 'correct_act', corrected_values, recorded['act'])))
    assert corrected['revision'] == 3 and corrected['act']['revision'] == 2
    assert corrected['act']['id'] == recorded['act']['id']
    assert corrected['act']['previous'] == {
        'revision': 2, 'capture_digest': recorded['receipt']['capture_digest'],
    }
    assert request('GET', path + '/revisions/2') == recorded
    assert corrected['sources'] == first['sources']
    written = act_values('written', support)
    written['kind'] = 'admission'
    appeal = submit(route, prepare(route, change(appeal_first, 'record_act', written), tokens['litigator']), tokens['litigator'])
    assert appeal['act']['values']['mode']['value'] == 'written'
    assert appeal['values'] == appeal_first['values']

    correction = change(corrected, 'correct')
    correction['change']['values']['grounds'] = 'Explicit organizational correction'
    stale = prepare(route, correction)
    winner = copy.deepcopy(correction)
    winner['operation_id'] = str(uuid4())
    fourth = submit(route, prepare(route, winner))
    submit(route, stale, expected=409, code='procedural_resource_revision_conflict')
    assert fourth['revision'] == 4 and fourth['act'] is None
    archived = submit(route, prepare(route, change(fourth, 'archive')))
    assert archived['revision'] == 5 and archived['status'] == 'archived' and archived['act'] is None
    assert archived['values'] == fourth['values'] and archived['sources'] == fourth['sources']
    prepare(route, change(archived, 'record_act', oral), expected=409, code='procedural_resource_archived')
    assert [row['id'] for row in request('GET', collection + '?status=archived')['resources']] == [first['id']]
    active = submit(route, prepare(route, change(archived, 'reactivate')))
    assert active['revision'] == 6 and active['status'] == 'active'
    assert active['values'] == archived['values'] and active['sources'] == archived['sources']
    assert active['act'] is None
    assert request('GET', path + '/revisions/1') == first
    prepare(route, change(active, 'reactivate'), expected=409, code='procedural_resource_state_unchanged')

    other = request('POST', '/api/v1/cases', {'title': 'Resource isolated case',
                    'reference': 'RESOURCE-FOREIGN'}, 201)
    other_route = '/api/v1/cases/' + other['id']
    request('GET', base(other_route) + '/' + first['id'], expected=404, code='procedural_resource_not_found')
    prepare(other_route, registration(parent, support), expected=404, code='procedural_fact_reference_not_found')
    request('GET', base(other_route), expected=404, token=tokens['litigator'], code='case_not_found')
    request('GET', path + '/revisions/99', expected=404, code='procedural_resource_not_found')
    first_page = request('GET', collection + '?limit=1')
    assert first_page['has_more'] and len(first_page['resources']) == 1
    tail = request('GET', collection + '?limit=1&after_id=' + first_page['next_after_id'])
    assert not tail['has_more'] and len(tail['resources']) == 1
    assert {first_page['resources'][0]['id'], tail['resources'][0]['id']} == {first['id'], appeal['id']}
    assert [row['id'] for row in request('GET', collection + '?kind=appeal')['resources']] == [appeal['id']]
    final = authorization(route, active, users, tokens)
    assert final['revision'] == 7
    history = request('GET', path + '/history?limit=2')
    assert [row['revision'] for row in history['revisions']] == [7, 6]
    assert history['has_more'] and history['next_before_revision'] == 6
    assert request('GET', path + '/history?before_revision=6')['revisions'][-1] == first
    assert request('GET', route + '/resolutions/' + parent['id'] + '/history') == initial_facts
    assert request('GET', route + '/stage') == initial_stage
    request('POST', '/api/v1/auth/logout', expected=204, token=tokens['litigator'])
    request('GET', path, expected=401, token=tokens['litigator'], code='invalid_session')
    paths = exact_paths(route, final) + exact_paths(route, appeal)
    paths += [collection, collection + '?kind=revocation', collection + '?kind=appeal',
              collection + '?status=archived', path + '/history?limit=2',
              route + '/resolutions/' + parent['id'] + '/revisions/1', route + '/stage']
    records = {item: request('GET', item) for item in dict.fromkeys(paths)}
    STATE.write_text(json.dumps({'records': records}, sort_keys=True), encoding='utf-8')
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Resource API passed: revocation and appeal, oral and written acts, exact corrections, archive/reactivate, four roles, replay/conflicts and closure.')
    print('Resource restore capture: ' + str(len(records)) + ' exact HTTP responses.')


def restore():
    records = json.loads(STATE.read_text(encoding='utf-8'))['records']
    for path, expected in records.items():
        assert request('GET', path) == expected, 'Restored resource response differs: ' + path
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Resource restore passed: ' + str(len(records)) + ' exact responses, original act captures and receipts.')


if __name__ == '__main__':
    {'capture': capture, 'restore': restore}[sys.argv[1]]()
