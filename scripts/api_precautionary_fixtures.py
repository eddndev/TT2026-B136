"""Native precautionary fixtures using the disposable HTTP acceptance helpers."""
import copy
from datetime import datetime, timedelta, timezone
import json
from uuid import uuid4

from api_procedural_facts_support import TOKEN, WORK, request, upload
from api_resource_hearings_support import enroll, login
from api_hearing_derived_deadline_contract import digest

STATE = WORK / 'precautionary-api-state.json'
COLLECTIONS = {
    'hearing': 'precautionary-hearings',
    'decision': 'measure-decisions',
    'administrative': 'measure-administrative-operations',
}
ERRORS = {
    'hearing': 'precautionary_hearing',
    'decision': 'measure_decision',
    'administrative': 'measure_administrative',
}


def route(case_id):
    return '/api/v1/cases/' + case_id


def collection(scenario, kind):
    return route(scenario['case_id']) + '/' + COLLECTIONS[kind]


def save(value):
    STATE.write_text(json.dumps(value, sort_keys=True, ensure_ascii=True), encoding='ascii')
    STATE.chmod(0o600)


def actor_token(scenario, role):
    return TOKEN if role == 'owner' else scenario['accounts'][role]['token']


def context(scenario):
    return request('GET', route(scenario['case_id']) + '/precautionary-context')['expectation']


def subject(scenario, name):
    path = route(scenario['case_id']) + '/participants/proposals'
    support = {**scenario['support'], 'locator': 'Page 1'}
    unknown = {'state': 'unknown', 'reason': 'Not declared in the synthetic support'}
    review = request('POST', path + '/review', {
        'subject': {'operation': 'create', 'values': {
            'kind': 'natural_person', 'name': {'state': 'known', 'value': name},
            'curp': unknown, 'identity_support': support}},
        'participant': {'operation': 'create'},
        'role': {'organization': None, 'legal_status': None,
                 'profile': {'kind': 'defendant', 'custody': unknown}, 'role_support': support},
        'certificate_base64': None,
    })
    prepared = {
        'proposal': review['proposal'],
        'review': {'directory_stamp': review['directory_stamp'],
                   'selection_reason': 'Synthetic identity declared by the selected support',
                   'different': [{'candidate': row['reference'],
                                  'reason': 'Different synthetic identity in this support',
                                  'support': support} for row in review['candidates']]},
        'certificate_base64': None,
    }
    draft = request('POST', path + '/prepare', prepared)
    result = request('POST', path + '/commit', {
        'prepared': prepared, 'signature_base64': None}, expected=201)
    assert result['submission_digest'] == draft['submission_digest']
    return {'participant': result, 'subject': result['subject']}


def fixture():
    case = request('POST', '/api/v1/penal-cases', {
        'title': 'Declared precautionary acceptance', 'reference': 'PRECAUTIONARY-API',
        'profile': {'nuc': 'PC-NUC', 'nuc_authority': 'Synthetic authority',
                    'judicial_case_number': 'PC-CJ', 'judicial_authority': 'Synthetic court',
                    'offenses': ['Synthetic offense'], 'general_information': None,
                    'complementary_identifiers': None}}, expected=201)
    case_id = case['id']
    owner = request('GET', '/api/v1/auth/me')
    request('PUT', route(case_id) + '/members/' + owner['id'], expected=204)
    scenario = {'case_id': case_id, 'owner': owner,
                'support': upload(route(case_id), 'pdf', 'precautionary-support.pdf')}
    scenario['people'] = [subject(scenario, name) for name in ['Amber Solis', 'Zeno Fuentes']]
    scenario['accounts'] = {role: enroll('precautionary-' + role, role, case_id)
                           for role in ['litigator', 'paralegal', 'client']}
    scenario['accounts']['outside'] = enroll('precautionary-outside', 'litigator', case_id, False)
    token = scenario['accounts']['litigator']['token']
    prefs = request('GET', '/api/v1/alert-preferences', token=token)['preferences']
    values = copy.deepcopy(prefs['values'])
    values['hearing_upcoming'] = {'lead_hours': [48],
                                  'channels': {'internal': True, 'email': False}}
    response = request('PUT', '/api/v1/alert-preferences', {
        'operation_id': str(uuid4()), 'expected_revision': prefs['revision'], 'values': values,
    }, token=token)
    assert response['preferences']['email_transport'] == 'disabled'
    scenario['preferences'] = response['preferences']
    return scenario


def at(hours):
    return (datetime.now(timezone.utc) + timedelta(hours=hours)).replace(
        microsecond=0).isoformat().replace('+00:00', 'Z')


def hearing(scenario, target=None):
    person = scenario['people'][0]['participant']
    return {
        'case_id': scenario['case_id'], 'operation_id': str(uuid4()), 'hearing_id': str(uuid4()),
        'change': {'action': 'schedule', 'context': context(scenario), 'values': {
            'purpose': 'review' if target else 'imposition', 'scheduled_at': at(36),
            'modality': 'in_person', 'venue': 'Synthetic hearing court',
            'note': 'Declared appointment without automatic legal effects',
            'participants': [{'participant_id': person['id'], 'revision': person['revision']}],
            'scheduling_basis': {'statement': 'Express scheduling declaration',
                                 'support': scenario['support'], 'locator': 'Page 1'},
            'review_targets': [target['reference']] if target else []}},
    }


def decision(scenario, scheduled):
    person = scenario['people'][0]['subject']
    capture = scheduled['capture']
    return {
        'case_id': scenario['case_id'], 'operation_id': str(uuid4()), 'decision_id': str(uuid4()),
        'context': context(scenario),
        'values': {'authority': 'Synthetic court',
                   'declared_at': {'precision': 'unknown', 'reason': 'No decision time declared'},
                   'justification': 'Express measure declaration', 'support': scenario['support'],
                   'locator': 'Page 2'},
        'anchor': {'kind': 'precautionary',
                   'hearing_id': capture['review']['command']['hearing_id'],
                   'revision': capture['review']['result_revision'],
                   'capture_digest': capture['capture_digest']},
        'outcome': {'kind': 'changes', 'effects': [{'action': 'impose', 'proposal': {
            'id': str(uuid4()), 'values': {
                'subject': {key: person[key] for key in ['id', 'revision', 'values_digest']},
                'kind': 'periodic_appearance', 'conditions': 'Appear each Friday as declared',
                'validity': {'start': {'precision': 'date', 'year': 2026, 'month': 1,
                                       'day': 2, 'offset_seconds': None},
                             'statement': 'Declared validity without inferred expiry', 'end': None},
                'supervision': {'kind': 'unknown', 'reason': 'Supervisor not declared'}}}}]},
    }


def administrative(scenario, target, replacement=False):
    values = target['record']['capture']['result']['values']
    action = {'kind': 'correct', 'values': {
        'conditions': 'Appear on Friday at the corrected declared venue',
        'validity': copy.deepcopy(values['validity']),
        'supervision_text': values['supervision']['reason']}}
    if replacement:
        person = scenario['people'][1]['subject']
        action = {'kind': 'replace_entered_in_error', 'replacement_id': str(uuid4()),
                  'subject': {key: person[key] for key in ['id', 'revision', 'values_digest']}}
    return {'case_id': scenario['case_id'], 'operation_id': str(uuid4()),
            'target': target['reference'], 'context': context(scenario),
            'reason': 'Express administrative declaration without a new judicial effect',
            'action': action}


def review(prepared, kind):
    return prepared['review'] if kind == 'decision' else prepared


def payload(prepared, kind):
    value = review(prepared, kind)
    return {'command': value['command'],
            'expected_submission_digest': value['submission_digest'],
            'expected_review_digest': value['review_digest']}


def operation_path(scenario, kind, command):
    path = collection(scenario, kind)
    return path + ('/' if kind == 'administrative' else '/operations/') + command['operation_id']


def transact(scenario, kind, command, role='owner', reject_digests=False):
    path, token = collection(scenario, kind), actor_token(scenario, role)
    prepared = request('POST', path + '/prepare', command, token=token)
    value = review(prepared, kind)
    assert value['command'] == command
    for key in ['submission_digest', 'review_digest']:
        digest(value[key])
    assert request('POST', path + '/prepare', command, token=token) == prepared
    original = operation_path(scenario, kind, command)
    request('GET', original, expected=404, token=token, code=ERRORS[kind] + '_not_found')
    body = payload(prepared, kind)
    if reject_digests:
        for field, suffix in [('expected_submission_digest', 'submission_mismatch'),
                              ('expected_review_digest', 'review_mismatch')]:
            request('POST', path + '/submit', {**body, field: '0' * 64}, expected=409,
                    token=token, code=ERRORS[kind] + '_' + suffix)
            request('GET', original, expected=404, token=token, code=ERRORS[kind] + '_not_found')
    operation = request('POST', path + '/submit', body, expected=201, token=token)
    captured = operation['group']['review'] if kind == 'decision' else operation['capture']['review']
    assert captured == value
    assert request('GET', original, token=token) == operation
    assert request('POST', path + '/submit', body, expected=201, token=token) == operation
    return {'kind': kind, 'role': role, 'prepared': prepared, 'operation': operation}


def replay(scenario, item):
    kind = item['kind']
    value = review(item['prepared'], kind)
    path, token = collection(scenario, kind), actor_token(scenario, item['role'])
    assert request('GET', operation_path(scenario, kind, value['command']), token=token) == item['operation']
    assert request('POST', path + '/prepare', value['command'], token=token) == item['prepared']
    assert request('POST', path + '/submit', payload(item['prepared'], kind),
                   expected=201, token=token) == item['operation']
