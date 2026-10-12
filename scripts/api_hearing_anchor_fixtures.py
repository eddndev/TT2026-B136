"""Synthetic hearing-end fixtures using the existing compound HTTP workflow."""
import copy
from uuid import uuid4

from api_hearing_derived_deadline_fixtures import command as compound_command
from api_hearing_derived_deadline_fixtures import persist, request, route

REQUIREMENT = {'kind': 'qualified', 'purpose': 'hearing_end', 'family': 'hearing_result'}
END = {'precision': 'second', 'year': 2026, 'month': 1, 'day': 6,
       'hour': 15, 'minute': 45, 'second': 12, 'offset_seconds': -21600}
INSTANT = {'unix_seconds': 1767735912, 'nanosecond': 0, 'offset_seconds': -21600}
DUE = {**INSTANT, 'offset_seconds': 0}
VARIANTS = ['exact', 'missing_qualification', 'date_only']


def fixture():
    actor = request('GET', '/api/v1/auth/me')
    actor = {key: actor[key] for key in ['id', 'email', 'role']}
    assert actor['role'] == 'owner', 'The disposable fixture requires its current Owner token'
    marker = uuid4().hex[:12]
    case = request('POST', '/api/v1/penal-cases', {
        'title': 'Synthetic exact hearing end ' + marker, 'reference': 'HEARING-ANCHOR-' + marker,
        'profile': {'nuc': 'HA-NUC-' + marker, 'nuc_authority': 'Synthetic authority',
                    'judicial_case_number': 'HA-CJ-' + marker,
                    'judicial_authority': 'Synthetic court', 'offenses': ['Synthetic offense'],
                    'general_information': None, 'complementary_identifiers': None}}, expected=201)
    case_id = case['id']
    request('PUT', route(case_id) + '/members/' + actor['id'], expected=204)
    reference = str(uuid4())
    definition = {
        'title': 'Synthetic exact declared hearing end',
        'description': 'Exact instant fixture without a duration or legal applicability claim',
        'scope': {'kind': 'case', 'case_id': case_id},
        'references': [{'id': reference, 'title': 'Synthetic hearing-end declaration',
            'issuer': 'Fixture', 'official_url': 'https://example.org/synthetic',
            'published_on': None, 'consulted_on': '2026-01-06', 'locator': 'Declared end'}],
        'trigger': copy.deepcopy(REQUIREMENT),
        'template': {'kind': 'fixed', 'rule': {'kind': 'at_anchor'}},
        'completion': {'kind': 'arithmetic_instant'},
        'conditions': [{'id': str(uuid4()), 'statement': 'Explicit synthetic applicability',
                        'reference_ids': [reference]}],
        'examples': [{'id': str(uuid4()), 'anchor': copy.deepcopy(END),
            'ordered_quantity': None, 'calendar': None,
            'expected': {'kind': 'arithmetic', 'outcome': {
                'kind': 'instant_candidate', 'instant': copy.deepcopy(INSTANT)}},
            'reference_ids': [reference], 'locator': 'Exact end retains its declared offset'}],
    }
    profile = persist(route(case_id) + '/deadline-profiles', {
        'profile_id': str(uuid4()), 'operation_id': str(uuid4()), 'change': {
            'action': 'publish', 'expected_revision': 0, 'definition': definition}})
    assert profile['definition'] == definition
    return {'case_id': case_id, 'actor': actor, 'profile': profile}


def declared_end(variant):
    assert variant in VARIANTS
    if variant == 'date_only':
        return {key: END[key] for key in ['year', 'month', 'day', 'offset_seconds']} | {
            'precision': 'date'}
    return copy.deepcopy(END)


def command(scenario, variant):
    assert variant in VARIANTS
    value = compound_command(scenario)
    definition = value['deadline']['change']['definition']
    definition['title'] = 'Synthetic declared hearing end: ' + variant
    definition['input']['selection']['qualification'] = None if variant == 'missing_qualification' else {
        'purpose': 'hearing_end', 'at': declared_end(variant),
        'statement': 'Express end of the selected hearing; independent of its event time',
        'locator': 'Synthetic hearing result, declared end'}
    return value


def expected_result(variant):
    assert variant in VARIANTS
    result = {'requirement': copy.deepcopy(REQUIREMENT), 'rule': {'kind': 'at_anchor'},
              'arithmetic': None, 'due_at': None, 'blocks': []}
    if variant == 'missing_qualification':
        block = {'kind': 'missing_qualification', 'purpose': 'hearing_end'}
        result['trigger_outcome'] = {'kind': 'blocked', 'block': block}
        result['blocks'] = [{'kind': 'trigger', 'block': block}]
        return result
    anchor = declared_end(variant)
    result['trigger_outcome'] = {'kind': 'extracted', 'at': anchor}
    arithmetic = {'rule': {'kind': 'at_anchor'}, 'anchor': anchor, 'trace': []}
    if variant == 'date_only':
        block = {'kind': 'insufficient_precision', 'observed': 'date'}
        arithmetic['outcome'] = {'kind': 'blocked', 'block': block}
        result['blocks'] = [{'kind': 'arithmetic', 'block': block}]
    else:
        arithmetic['outcome'] = {'kind': 'instant_candidate', 'instant': copy.deepcopy(INSTANT)}
        arithmetic['trace'] = [{'kind': 'at_anchor', 'instant': copy.deepcopy(INSTANT)}]
        result['due_at'] = copy.deepcopy(DUE)
    result['arithmetic'] = arithmetic
    return result
