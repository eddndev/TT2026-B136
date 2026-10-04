"""HTTP fixtures and exact evidence assertions for resource hearing acceptance."""
import copy
from datetime import datetime, timedelta, timezone
import json
from uuid import uuid4

import api_procedural_facts_support as facts
import api_procedural_resources_support as resources
import api_resource_activities_support as activities

TOKEN, WORK, request = facts.TOKEN, facts.WORK, facts.request
STATE = WORK / 'resource-hearings-api-state.json'


def route(case_id):
    return '/api/v1/cases/' + case_id


def base(resource):
    return activities.base(resource) + '/resource-hearings'


def reference(row):
    return {'id': row['id'], 'revision': row['revision'],
            'capture_digest': row['receipt']['capture_digest']}


def login(account, index):
    challenge = request('POST', '/api/v1/auth/login', {
        'email': account['email'], 'password': account['password'],
    })
    return request('POST', '/api/v1/auth/mfa/recovery', {
        'challenge_token': challenge['challenge_token'],
        'code': account['recovery_codes'][index],
    })['access_token']


def enroll(label, role, case_id, member=True):
    account = {'email': 'resource-hearings-' + label + '@example.com',
               'password': 'synthetic resource hearing fixture password', 'role': role}
    result = request('POST', '/api/v1/users', account, expected=201)
    account.update(id=result['user']['id'], recovery_codes=result['recovery_codes'])
    account['token'] = login(account, 0)
    if member:
        request('PUT', route(case_id) + '/members/' + account['id'], expected=204)
    return account


def fixture():
    case = request('POST', '/api/v1/penal-cases', {
        'title': 'Own resource hearing acceptance', 'reference': 'RESOURCE-HEARING-API',
        'profile': {'nuc': 'RH-NUC', 'nuc_authority': 'Declared authority',
                    'judicial_case_number': 'RH-CJ', 'judicial_authority': 'Declared court',
                    'offenses': ['Synthetic offense'], 'general_information': None,
                    'complementary_identifiers': None}}, expected=201)
    case_id = case['id']
    actor = request('GET', '/api/v1/auth/me')
    request('PUT', route(case_id) + '/members/' + actor['id'], expected=204)
    first, head, act = activities.own_resource({'case_id': case_id}, with_act=True)
    person = request('POST', route(case_id) + '/participants', {
        'display_name': 'Captured resource hearing participant',
        'procedural_role': 'Declared recurrent person'}, expected=201)
    accounts = {role: enroll(role, role, case_id)
                for role in ['litigator', 'paralegal', 'client']}
    accounts['outside'] = enroll('outside', 'litigator', case_id, member=False)
    # Keep the shared Owner defaults intact for the independent alert campaign.
    prefs = request('GET', '/api/v1/alert-preferences', token=accounts['litigator']['token'])['preferences']
    values = copy.deepcopy(prefs['values'])
    values['deadline_upcoming']['channels'] = {'internal': False, 'email': False}
    values['hearing_upcoming'] = {'lead_hours': [48, 24],
                                  'channels': {'internal': True, 'email': False}}
    saved = request('PUT', '/api/v1/alert-preferences', {
        'operation_id': str(uuid4()), 'expected_revision': prefs['revision'], 'values': values,
    }, token=accounts['litigator']['token'])['preferences']
    assert saved['values'] == values and saved['email_transport'] == 'disabled'
    return {'case_id': case_id, 'owner': actor, 'resource': first, 'head': head,
            'act': act, 'participant': person, 'accounts': accounts, 'preferences': saved}


def command(scenario, hours=36, with_act=True):
    first, head, act = scenario['resource'], scenario['head'], scenario['act']
    support = first['sources']['supports'][0]
    scheduled = (datetime.now(timezone.utc) + timedelta(hours=hours)).replace(microsecond=0)
    return {
        'case_id': first['case_id'], 'resource_id': first['id'],
        'operation_id': str(uuid4()), 'hearing_id': str(uuid4()), 'association_id': str(uuid4()),
        'expected_resource_revision': head['revision'], 'resource': reference(first),
        'act': {'id': act['act']['id'], 'revision': act['act']['revision'],
                'resource_revision': act['revision'],
                'capture_digest': act['receipt']['capture_digest']} if with_act else None,
        'values': {'kind': 'written_revocation',
                   'scheduled_at': scheduled.isoformat().replace('+00:00', 'Z'),
                   'modality': 'in_person', 'venue': 'Declared resource hearing court',
                   'note': 'Synthetic appointment without inferred legal effects',
                   'participants': [{'participant_id': scenario['participant']['id'],
                                     'revision': scenario['participant']['revision']}],
                   'scheduling_basis': {'statement': 'Express scheduling support declaration',
                       'support': {k: support[k] for k in ['document_id', 'version', 'digest']}}},
    }


def prepare(collection, value, token=TOKEN, expected=200, code=None):
    return request('POST', collection + '/prepare', value, expected, token, code)


def submit(collection, draft, token=TOKEN, expected=201, code=None):
    return request('POST', collection + '/submit', {
        'command': draft['command'], 'expected_submission_digest': draft['submission_digest'],
    }, expected, token, code)


def exact(collection, value):
    return collection + '/' + value['hearing']['id'] + '/revisions/1'


def creation(result, draft, scenario):
    assert set(result) == {'hearing', 'association', 'origin', 'submission_digest'}
    hearing, link, selected = result['hearing'], result['association'], draft['command']
    assert draft['resource'] == scenario['resource']
    assert draft['act'] == (scenario['act'] if selected['act'] else None)
    assert draft['observed_resource_head'] == reference(scenario['head'])
    assert hearing['case_id'] == link['case_id'] == selected['case_id']
    assert hearing['resource_id'] == link['resource_id'] == selected['resource_id']
    assert hearing['id'] == selected['hearing_id']
    assert hearing['association_id'] == link['id'] == selected['association_id']
    assert hearing['revision'] == link['revision'] == 1
    assert hearing['operation_id'] == link['receipt']['operation_id'] == selected['operation_id']
    for field in ['resource', 'act', 'values', 'expected_resource_revision']:
        assert hearing[field] == selected[field], field
    assert hearing['submission_digest'] == result['submission_digest'] == draft['submission_digest']
    assert hearing['sources'] == {'resource': draft['resource'], 'act': draft['act'], **draft['sources']}
    assert hearing['recorded_by'] == link['recorded_by'] == draft['recorded_by']
    assert hearing['recorded_at'] == link['recorded_at']
    assert hearing['recorded_administration'] == link['recorded_administration'] == draft['observed_administration']
    assert hearing['recorded_resource_head'] == link['recorded_resource_head'] == draft['observed_resource_head']
    assert result['origin'] == {
        'case_id': hearing['case_id'], 'resource_id': hearing['resource_id'],
        'hearing_id': hearing['id'], 'association_id': link['id'],
        'operation_id': hearing['operation_id'], 'submission_digest': hearing['submission_digest'],
        'capture_digest': hearing['capture_digest'],
    }
    assert link['status'] == 'linked'
    assert link['selection'] == {'resource': selected['resource'], 'act': selected['act'],
        'target': {'kind': 'resource_hearing', 'id': hearing['id'], 'revision': 1,
                   'capture_digest': hearing['capture_digest']}}
    assert link['sources'] == {'resource': draft['resource'], 'act': draft['act'],
                              'target': {'kind': 'resource_hearing', 'record': hearing}}
    assert 'status' not in hearing and 'scheduling_context' not in hearing


def absent(collection, value):
    request('GET', collection + '/' + value['hearing_id'] + '/revisions/1', expected=404,
            code='resource_activity_not_found')
    request('GET', collection.removesuffix('/resource-hearings') + '/' + value['association_id'],
            expected=404, code='resource_activity_not_found')


def listing(collection, results, token=TOKEN):
    expected = sorted(results, key=lambda item: item['hearing']['id'])
    page = request('GET', collection + '?limit=1', token=token)
    assert page['case_id'] == expected[0]['hearing']['case_id']
    assert page['resource_id'] == expected[0]['hearing']['resource_id']
    assert page['items'] == expected[:1] and page['has_more']
    assert page['next_after_id'] == expected[0]['hearing']['id']
    tail = request('GET', collection + '?limit=1&after_id=' + page['next_after_id'], token=token)
    assert tail['items'] == expected[1:] and not tail['has_more'] and tail['next_after_id'] is None
    all_rows = request('GET', collection, token=token)
    assert all_rows['items'] == expected and not all_rows['has_more']


def save(value):
    STATE.write_text(json.dumps(value, sort_keys=True), encoding='ascii')
    STATE.chmod(0o600)
