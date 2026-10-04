"""Independent synthetic hearing arithmetic over existing disposable HTTP helpers."""
import copy
from uuid import uuid4

from api_deadlines_support import qualified_time, register
from api_hearing_results_demo import command as result_command
from api_resource_hearings_support import enroll, login
import api_procedural_facts_support as facts

TOKEN, WORK, request = facts.TOKEN, facts.WORK, facts.request


def route(case_id):
    return '/api/v1/cases/' + case_id


def base(command):
    return route(command['case_id']) + '/hearings/' + command['result']['hearing_id'] + '/results/derived-deadline'


def persist(collection, command):
    draft = request('POST', collection + '/prepare', command)
    return request('POST', collection, {'command': draft['command'],
                   'expected_submission_digest': draft['submission_digest']}, expected=201)


def fixture():
    actor = request('GET', '/api/v1/auth/me')
    actor = {key: actor[key] for key in ['id', 'email', 'role']}
    case = request('POST', '/api/v1/penal-cases', {
        'title': 'Synthetic compound hearing deadline', 'reference': 'HEARING-DERIVED-API',
        'profile': {'nuc': 'HD-NUC', 'nuc_authority': 'Synthetic authority',
                    'judicial_case_number': 'HD-CJ', 'judicial_authority': 'Synthetic court',
                    'offenses': ['Synthetic offense'], 'general_information': None,
                    'complementary_identifiers': None}}, expected=201)
    case_id = case['id']
    request('PUT', route(case_id) + '/members/' + actor['id'], expected=204)
    accounts = {role: enroll('derived-' + role, role, case_id)
                for role in ['litigator', 'paralegal', 'client']}
    accounts['outside'] = enroll('derived-outside', 'litigator', case_id, member=False)
    reference, condition = str(uuid4()), str(uuid4())
    definition = {
        'title': 'Synthetic hours from declared hearing event',
        'description': 'Arithmetic fixture without legal approval or inferred hearing end',
        'scope': {'kind': 'case', 'case_id': case_id},
        'references': [{'id': reference, 'title': 'Synthetic arithmetic', 'issuer': 'Fixture',
            'official_url': 'https://example.org/synthetic', 'published_on': None,
            'consulted_on': '2026-01-06', 'locator': 'Explicit elapsed-hour example'}],
        'trigger': {'kind': 'source_field', 'field': 'hearing_session_event_time'},
        'template': {'kind': 'fixed', 'rule': {'kind': 'elapsed_hours', 'quantity': 24}},
        'completion': {'kind': 'arithmetic_instant'},
        'conditions': [{'id': condition, 'statement': 'Explicit synthetic applicability',
                        'reference_ids': [reference]}],
        'examples': [{'id': str(uuid4()), 'anchor': qualified_time(), 'ordered_quantity': None,
            'calendar': None, 'expected': {'kind': 'arithmetic', 'outcome': {
                'kind': 'instant_candidate', 'instant': {
                    'unix_seconds': 1767817807, 'nanosecond': 0, 'offset_seconds': 0}}},
            'reference_ids': [reference], 'locator': 'Twenty-four elapsed hours'}],
    }
    profile = persist(route(case_id) + '/deadline-profiles', {
        'profile_id': str(uuid4()), 'operation_id': str(uuid4()), 'change': {
            'action': 'publish', 'expected_revision': 0, 'definition': definition}})
    return {'case_id': case_id, 'actor': actor, 'accounts': accounts, 'profile': profile}


def command(scenario, blocked=False):
    collection = route(scenario['case_id'])
    context = request('GET', collection + '/hearings/context')
    hearing = persist(collection + '/hearings', {
        'operation_id': str(uuid4()), 'hearing_id': str(uuid4()), 'change': {
            'action': 'schedule', 'expected_revision': 0,
            'expected_case_revision': context['case_revision'],
            'expected_stage_revision': context['stage_revision'], 'values': {
                'kind': 'initial', 'scheduled_at': '2026-01-06T14:30:07-06:00',
                'modality': 'in_person', 'venue': 'Synthetic hearing court',
                'note': None, 'participants': [], 'conviction_basis': None}}})
    event = {'precision': 'date', 'date': '2026-01-06', 'offset': '-06:00'} if blocked else {
        'precision': 'instant', 'at': '2026-01-06T14:30:07-06:00'}
    result = result_command(hearing['id'], 1, {
        'occurrence': 'occurred', 'extent': 'concluded', 'event_time': event,
        'summary': 'Synthetic declared event, not an inferred hearing end',
        'attendees': [], 'agreements': [],
        'provenance': {'kind': 'operator_note', 'reference': None, 'support': None}})
    deadline = register(scenario['case_id'], scenario['profile'],
                        {'id': result['result_id'], 'revision': 1}, scenario['actor']['id'])
    definition = deadline['change']['definition']
    definition['title'] = 'Blocked declared consequence' if blocked else 'Calculable declared consequence'
    definition['input']['selection']['source']['value'] = {
        'family': 'hearing_result', 'hearing_id': hearing['id'],
        'result_id': result['result_id'], 'revision': 1, 'agreement_id': None}
    definition['input']['qualification']['locator'] = 'Explicit declared hearing event'
    deadline['change']['tracking']['profile'] = 'fixed'
    return {'case_id': scenario['case_id'], 'result': result, 'deadline': deadline}


def prepare(command, token=TOKEN, expected=200, code=None):
    return request('POST', base(command) + '/prepare', command, expected, token, code=code)


def submit(ready, token=TOKEN, expected=201, code=None):
    return request('POST', base(ready['command']) + '/submit', {
        'command': ready['command'], 'expected_review_digest': ready['review_digest']},
        expected, token, code=code)


def changed(command):
    value = copy.deepcopy(command)
    value['deadline']['change']['definition']['title'] += ' altered'
    return value
