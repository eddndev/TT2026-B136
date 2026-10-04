"""Exercise current authorization without replacing retained hearing evidence."""
import copy
from uuid import uuid4

from api_resource_hearings_support import (
    TOKEN, absent, base, command, exact, listing, prepare, request, resources, route, submit,
)
from api_resource_hearings_observation import agenda_page, own_alerts, period


def permissions(scenario, results, drafts, litigator_alert):
    collection = base(scenario['resource'])
    accounts, case = scenario['accounts'], route(scenario['case_id'])
    for token in [TOKEN, accounts['litigator']['token'], accounts['paralegal']['token']]:
        listing(collection, results, token)
        for result in results:
            assert request('GET', exact(collection, result), token=token) == result
            assert request('GET', collection + '/' + result['hearing']['id'], token=token) == result
    for token, status, code in [('', 401, 'invalid_session'),
                                (accounts['client']['token'], 403, 'permission_denied'),
                                (accounts['outside']['token'], 404, 'case_not_found')]:
        for path in [collection, exact(collection, results[0])]:
            request('GET', path, expected=status, token=token, code=code)
    value = command(scenario)
    for role in ['paralegal', 'client']:
        prepare(collection, value, accounts[role]['token'], 403, 'permission_denied')
        submit(collection, drafts[0], accounts[role]['token'], 403, 'permission_denied')

    pending = prepare(collection, value, accounts['litigator']['token'])
    member = case + '/members/' + accounts['litigator']['id']
    request('DELETE', member, expected=204)
    submit(collection, pending, accounts['litigator']['token'], 404, 'case_not_found')
    submit(collection, drafts[1], accounts['litigator']['token'], 404, 'case_not_found')
    absent(collection, value)
    hidden = request('GET', exact(collection, results[0]), expected=404,
                     token=accounts['litigator']['token'], code='case_not_found')
    missing = request('GET', exact(collection, results[0]).replace(scenario['case_id'], str(uuid4())),
                      expected=404, token=accounts['litigator']['token'], code='case_not_found')
    assert hidden == missing
    assert agenda_page(period(results), accounts['litigator']['token']) == []
    assert own_alerts(results[0], accounts['litigator']['token']) == []
    alert_path = '/api/v1/alerts/' + litigator_alert['id']
    request('GET', alert_path, expected=404, token=accounts['litigator']['token'], code='alert_not_found')
    request('POST', alert_path + '/read', {'operation_id': str(uuid4())}, expected=404,
            token=accounts['litigator']['token'], code='alert_not_found')
    request('PUT', member, expected=204)
    assert submit(collection, drafts[1], accounts['litigator']['token']) == results[1]


def conflict(scenario, results):
    collection = base(scenario['resource'])
    value = command(scenario)
    stale = prepare(collection, value)
    replacement = copy.deepcopy(scenario['head']['values'])
    replacement['title'] += ' corrected after hearing preparation'
    scenario['head'] = resources.submit(route(scenario['case_id']), resources.prepare(
        route(scenario['case_id']), resources.change(scenario['head'], 'correct', replacement)))
    assert scenario['head']['revision'] == 4
    submit(collection, stale, expected=409, code='resource_activity_resource_revision_conflict')
    absent(collection, value)
    listing(collection, results)


def retained_history(scenario, results, drafts):
    from api_resource_hearings_support import activities
    collection, case = base(scenario['resource']), route(scenario['case_id'])
    participant = scenario['participant']
    participant_path = case + '/participants/' + participant['id']
    request('PUT', participant_path, {
        'expected_revision': participant['revision'], 'display_name': 'Updated directory name',
        'procedural_role': 'Declared recurrent person', 'organization': None,
        'legal_status': None, 'directory_status': 'active',
    })
    assert request('GET', participant_path)['revision'] == participant['revision'] + 1
    current_person = request('GET', participant_path)
    scenario['participant'] = current_person
    fresh = command(scenario)
    pending = prepare(collection, fresh)
    archived = resources.submit(case, resources.prepare(case, resources.change(scenario['head'], 'archive')))
    assert archived['revision'] == 5 and archived['status'] == 'archived'
    scenario['head'] = archived
    prepare(collection, command(scenario), expected=409, code='resource_activity_resource_archived')
    submit(collection, pending, expected=409, code='resource_activity_resource_revision_conflict')
    absent(collection, fresh)
    link_base = activities.base(scenario['resource'])
    removed = activities.submit(link_base, activities.prepare(link_base,
        activities.unlink(results[0]['association'], archived)))
    assert removed['status'] == 'unlinked' and removed['revision'] == 2
    assert removed['sources'] == results[0]['association']['sources']
    admin = request('GET', case + '/administration')['administration']
    request('PUT', case + '/administrative-status', {
        'expected_revision': admin['revision'], 'administrative_status': 'closed',
    })
    prepare(collection, command(scenario), expected=409, code='case_closed')
    for result, draft, token in zip(results, drafts, [TOKEN, scenario['accounts']['litigator']['token']]):
        assert submit(collection, draft, token) == result
        assert request('GET', exact(collection, result)) == result
    assert request('GET', participant_path + '/revisions/1') == participant
    listing(collection, results)
    return removed
