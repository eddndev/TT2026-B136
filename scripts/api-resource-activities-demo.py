#!/usr/bin/env python3
"""Accept exact resource associations and compare restored HTTP evidence."""
import copy
import json
import sys
from uuid import uuid4
from api_resource_activities_support import (
    STATE, TOKEN, base, enroll, link, own_resource, prepare, receipt,
    request, resources, stable, submit, targets, unlink,
)


def authorization(first, head, hearing, act, original, original_draft, users, tokens):
    route = '/api/v1/cases/' + first['case_id']
    collection = base(first)
    path = collection + '/' + original['id']
    for token in [TOKEN, tokens['litigator'], tokens['paralegal']]:
        for suffix in ['', '/history', '/revisions/1']:
            request('GET', path + suffix, token=token)
        request('GET', collection, token=token)
    for suffix in ['', '/' + original['id'], '/' + original['id'] + '/history',
                   '/' + original['id'] + '/revisions/1']:
        request('GET', collection + suffix, expected=403, token=tokens['client'], code='permission_denied')
    command = link(first, head, hearing, 'hearing', act)
    draft = prepare(collection, command, tokens['litigator'])
    for role in ['paralegal', 'client']:
        prepare(collection, command, tokens[role], 403, 'permission_denied')
        submit(collection, draft, tokens[role], 403, 'permission_denied')
    request('DELETE', route + '/members/' + users['litigator'], expected=204)
    submit(collection, draft, tokens['litigator'], 404, 'case_not_found')
    submit(collection, original_draft, tokens['litigator'], 404, 'case_not_found')
    hidden = request('GET', path, expected=404, token=tokens['litigator'])
    missing = request('GET', '/api/v1/cases/' + str(uuid4())
                      + '/procedural-resources/' + first['id'] + '/activities/' + original['id'],
                      expected=404, token=tokens['litigator'])
    assert hidden == missing
    request('PUT', route + '/members/' + users['litigator'], expected=204)
    admin = request('GET', route + '/administration')['administration']
    request('PUT', route + '/administrative-status', {
        'expected_revision': admin['revision'], 'administrative_status': 'closed',
    })
    submit(collection, draft, tokens['litigator'], 409, 'case_closed')
    prepare(collection, command, expected=409, code='case_closed')
    assert submit(collection, original_draft, tokens['litigator']) == original
    assert request('GET', path + '/history', token=tokens['paralegal'])['revisions'] == [original]
    request('PUT', route + '/administrative-status', {
        'expected_revision': admin['revision'] + 1, 'administrative_status': 'active',
    })
    submit(collection, draft, tokens['litigator'], 409, 'resource_activity_submission_mismatch')
    request('POST', '/api/v1/auth/logout', expected=204, token=tokens['litigator'])
    request('GET', path, expected=401, token=tokens['litigator'], code='invalid_session')


def capture():
    hearing, deadline = targets()
    first, head, act = own_resource(hearing, with_act=True)
    deadline_first, deadline_head, _ = own_resource(deadline)
    collection, deadline_collection = base(first), base(deadline_first)
    route = '/api/v1/cases/' + first['case_id']
    users, tokens = {}, {}
    for role in ['litigator', 'paralegal', 'client']:
        users[role], tokens[role] = enroll(role, route)
    hearing_path = route + '/hearings/' + hearing['id']
    deadline_path = '/api/v1/cases/' + deadline['case_id'] + '/deadlines/' + deadline['id']
    original_hearing = request('GET', hearing_path)
    original_deadline = stable(request('GET', deadline_path))
    stage = request('GET', route + '/stage')
    command = link(first, head, hearing, 'hearing', act)
    assert request('GET', collection)['associations'] == []
    draft = prepare(collection, command, tokens['litigator'])
    assert prepare(collection, command, tokens['litigator']) == draft
    assert request('GET', collection)['associations'] == []
    assert draft['sources']['resource'] == first
    assert draft['sources']['act'] == act
    assert draft['sources']['target'] == {'kind': 'hearing', 'record': hearing}
    assert draft['selection']['resource']['revision'] == 1
    assert draft['selection']['act']['revision'] == 1
    assert draft['selection']['act']['resource_revision'] == 2
    assert head['revision'] == 3 and head['act']['revision'] == 2
    submit(collection, {**draft, 'submission_digest': '00' * 32}, tokens['litigator'], 409,
           'resource_activity_submission_mismatch')
    linked = submit(collection, draft, tokens['litigator'])
    receipt(linked, draft)
    assert submit(collection, draft, tokens['litigator']) == linked
    assert prepare(collection, command, tokens['litigator']) == draft
    path = collection + '/' + linked['id']
    view = request('GET', path + '/revisions/1')
    stable(view)
    assert view['association'] == linked
    assert view['current_target'] == {'kind': 'hearing', 'record': original_hearing}
    assert original_hearing['revision'] > hearing['revision']
    assert original_hearing['status'] == 'cancelled'
    changed_operation = copy.deepcopy(command)
    changed_operation['association_id'] = str(uuid4())
    prepare(collection, changed_operation, tokens['litigator'], 409, 'resource_activity_operation_conflict')

    plain_draft = prepare(collection, link(first, head, hearing, 'hearing'))
    plain = submit(collection, plain_draft)
    receipt(plain, plain_draft)
    assert plain['selection']['act'] is None and plain['sources']['act'] is None
    deadline_draft = prepare(deadline_collection, link(deadline_first, deadline_head, deadline, 'deadline'))
    deadline_link = submit(deadline_collection, deadline_draft)
    receipt(deadline_link, deadline_draft)
    deadline_view = request('GET', deadline_collection + '/' + deadline_link['id'])
    stable(deadline_view)
    assert deadline_view['association']['sources']['target']['record'] == deadline
    current = deadline_view['current_target']['record']
    assert current['revision'] > deadline['revision'] and current['status'] == 'retired'
    assert current['operational']['due_at'] is None
    assert current['calculation']['result']['due_at'] is not None
    assert current['operational']['freshness'] == 'not_checked'

    foreign = link(first, head, deadline, 'deadline')
    prepare(collection, foreign, expected=404)
    request('GET', deadline_collection + '/' + linked['id'], expected=404, code='resource_activity_not_found')
    request('GET', path + '/revisions/99', expected=404, code='resource_activity_not_found')
    authorization(first, head, hearing, act, linked, draft, users, tokens)

    stale = prepare(collection, link(first, head, hearing, 'hearing'))
    archived = resources.submit(route, resources.prepare(route, resources.change(head, 'archive')))
    assert archived['status'] == 'archived' and archived['revision'] == 4
    submit(collection, stale, expected=409, code='resource_activity_resource_revision_conflict')
    prepare(collection, link(first, archived, hearing, 'hearing'), expected=409,
            code='resource_activity_resource_archived')
    unlink_draft = prepare(collection, unlink(linked, archived))
    removed = submit(collection, unlink_draft)
    receipt(removed, unlink_draft)
    assert removed['status'] == 'unlinked' and removed['revision'] == 2
    assert removed['selection'] == linked['selection'] and removed['sources'] == linked['sources']
    assert submit(collection, unlink_draft) == removed
    assert request('GET', path + '/revisions/1')['association'] == linked
    history = request('GET', path + '/history?limit=1')
    assert history['revisions'] == [removed] and history['has_more']
    assert history['next_before_revision'] == 2
    assert request('GET', path + '/history?before_revision=2')['revisions'] == [linked]
    prepare(collection, unlink(removed, archived), expected=409, code='resource_activity_state_unchanged')

    first_page = request('GET', collection + '?limit=1&kind=hearing')
    stable(first_page)
    assert first_page['has_more'] and len(first_page['associations']) == 1
    tail = request('GET', collection + '?limit=1&kind=hearing&after_id=' + first_page['next_after_id'])
    stable(tail)
    assert not tail['has_more'] and len(tail['associations']) == 1
    assert {v['association']['id'] for v in first_page['associations'] + tail['associations']} == {linked['id'], plain['id']}
    for status, expected_id in [('linked', plain['id']), ('unlinked', linked['id'])]:
        page = request('GET', collection + '?status=' + status)
        assert [v['association']['id'] for v in page['associations']] == [expected_id]
    assert request('GET', collection + '?kind=deadline')['associations'] == []
    assert request('GET', hearing_path) == original_hearing
    assert stable(request('GET', deadline_path)) == original_deadline
    assert request('GET', route + '/stage') == stage
    paths = [collection, collection + '?status=linked', collection + '?status=unlinked',
             collection + '?kind=deadline', deadline_collection, hearing_path, deadline_path]
    for prefix, row in [(collection, removed), (collection, plain), (deadline_collection, deadline_link)]:
        item = prefix + '/' + row['id']
        paths += [item, item + '/history'] + [item + '/revisions/' + str(n)
                  for n in range(1, row['revision'] + 1)]
    paths += resources.exact_paths(route, archived)
    paths += resources.exact_paths('/api/v1/cases/' + deadline_first['case_id'], deadline_first)
    records = {p: stable(request('GET', p)) for p in dict.fromkeys(paths)}
    STATE.write_text(json.dumps({'records': records}, sort_keys=True), encoding='utf-8')
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Resource activity API passed: exact hearing/deadline and act captures, separate current heads, '
          'unlink/replay, history/pages, four roles, revocation, closure, archive and isolation.')


def restore():
    records = json.loads(STATE.read_text(encoding='utf-8'))['records']
    for path, expected in records.items():
        assert stable(request('GET', path)) == expected, 'Restored association response differs: ' + path
    assert request('GET', '/api/v1/audit/verify')['valid']
    print('Resource activity restore passed: ' + str(len(records))
          + ' responses, immutable captures/receipts and separately validated read instants.')


if __name__ == '__main__':
    {'capture': capture, 'restore': restore}[sys.argv[1]]()
