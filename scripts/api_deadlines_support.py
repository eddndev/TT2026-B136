"""Synthetic fixtures and bounded requests for persistent deadline acceptance."""
import copy
import json
import os
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen
from uuid import uuid4

BASE = os.environ['TT_DEADLINE_API_BASE_URL']
TOKEN = os.environ['TT_DEADLINE_API_TOKEN']
STATE = Path(os.environ['TT_DEADLINE_API_WORK_DIR']) / 'deadlines-api-state.json'


def request(method, path, body=None, expected=200, token=TOKEN, code=None):
    headers = {'Authorization': 'Bearer ' + token}
    if body is not None:
        body = json.dumps(body, ensure_ascii=True, separators=(',', ':')).encode('ascii')
        headers['Content-Type'] = 'application/json'
    try:
        response = urlopen(Request(BASE + path, data=body, headers=headers, method=method), timeout=60)
    except HTTPError as error:
        response = error
    with response:
        raw = response.read(4 * 1024 * 1024 + 1)
        assert len(raw) <= 4 * 1024 * 1024, 'Deadline response exceeds fixture budget'
        value = json.loads(raw) if raw else None
        actual = (value or {}).get('error', {}).get('code')
        assert response.status == expected, (method, path, response.status, expected, actual)
        if code:
            assert actual == code, (path, actual, code)
        return value


def enroll(role):
    email = 'persistent-deadline-' + role + '@example.com'
    password = 'synthetic deadline fixture password'
    row = request('POST', '/api/v1/users', {'email': email, 'password': password, 'role': role}, 201)
    challenge = request('POST', '/api/v1/auth/login', {'email': email, 'password': password})
    session = request('POST', '/api/v1/auth/mfa/recovery', {
        'challenge_token': challenge['challenge_token'], 'code': row['recovery_codes'][0],
    })
    return row['user']['id'], session['access_token']


def date(day=6):
    return {'precision': 'date', 'year': 2026, 'month': 1, 'day': day, 'offset_seconds': None}


def qualified_time():
    return {'precision': 'second', 'year': 2026, 'month': 1, 'day': 6,
            'hour': 14, 'minute': 30, 'second': 7, 'offset_seconds': -21600}


def profile(case_id, kind):
    ref, condition = str(uuid4()), str(uuid4())
    definition = {
        'title': 'Synthetic ' + kind + ' profile', 'description': 'Arithmetic fixture without legal claims',
        'scope': {'kind': 'case', 'case_id': case_id},
        'references': [{'id': ref, 'title': 'Synthetic reference', 'issuer': 'Fixture',
            'official_url': 'https://example.org/synthetic', 'published_on': None,
            'consulted_on': '2026-01-06', 'locator': 'Mathematical example'}],
        'trigger': {'kind': 'source_field', 'field': 'resolution_issued_at'},
        'template': {'kind': 'fixed', 'rule': {'kind': 'days', 'quantity': 1,
            'inclusion': 'on_anchor', 'basis': 'natural', 'final_day': 'preserve'}},
        'completion': {'kind': 'civil_cutoff', 'time': '17:30:00', 'offset_seconds': -21600,
            'from': '2026-01-01', 'through': '2026-12-31', 'channel': 'Declared filing channel', 'reference_id': ref},
        'conditions': [{'id': condition, 'statement': 'Operator declares applicability', 'reference_ids': [ref]}],
        'examples': [{'id': str(uuid4()), 'anchor': date(), 'ordered_quantity': None, 'calendar': None,
            'expected': {'kind': 'arithmetic', 'outcome': {'kind': 'civil_candidate', 'date': '2026-01-06'}},
            'reference_ids': [ref], 'locator': 'Synthetic arithmetic example'}],
    }
    if kind == 'monthly':
        definition['template']['rule'] = {'kind': 'civil_months', 'quantity': 1, 'final_day': 'preserve'}
        definition['examples'][0]['expected']['outcome']['date'] = '2026-02-06'
    if kind == 'hourly':
        definition['trigger'] = {'kind': 'qualified', 'family': 'resolution', 'purpose': 'ordered_period_start'}
        definition['template'] = {'kind': 'ordered', 'unit': {'kind': 'elapsed_hours'}, 'maximum': 72}
        definition['completion'] = {'kind': 'arithmetic_instant'}
        definition['examples'][0].update(anchor=qualified_time(), ordered_quantity=24,
            expected={'kind': 'arithmetic', 'outcome': {'kind': 'instant_candidate',
                'instant': {'unix_seconds': 1767817807, 'nanosecond': 0, 'offset_seconds': 0}}})
    command = {'profile_id': str(uuid4()), 'operation_id': str(uuid4()),
        'change': {'action': 'publish', 'expected_revision': 0, 'definition': definition}}
    route = '/api/v1/cases/' + case_id + '/deadline-profiles'
    draft = request('POST', route + '/prepare', command)
    return request('POST', route, {'command': command, 'expected_submission_digest': draft['submission_digest']}, 201)


def resolution(route):
    command = {'family': 'resolution', 'id': str(uuid4()), 'operation_id': str(uuid4()), 'change': {
        'action': 'record', 'expected_revision': 0, 'values': {'subtype': None,
            'class': {'kind': 'known', 'value': {'kind': 'order'}},
            'issuer': {'kind': 'known', 'value': 'Synthetic authority'}, 'issued_at': date(31),
            'summary': 'Synthetic deadline source', 'provenance': {'kind': 'operator_note', 'note': 'Fixture'}}}}
    draft = request('POST', route + '/resolutions/prepare', command)
    return request('POST', route + '/resolutions', {
        'command': command, 'expected_submission_digest': draft['submission_digest']}, 201)


def register(case_id, profile_row, source, responsible):
    return {'operation_id': str(uuid4()), 'deadline_id': str(uuid4()), 'change': {
        'action': 'register', 'expected_revision': 0,
        'tracking': {'profile': 'follow', 'source': 'follow', 'calendar': 'undetermined'},
        'definition': {
            'title': 'Declared deadline', 'profile': {'id': profile_row['id'], 'revision': profile_row['revision']},
            'responsible_id': responsible, 'input': {'selection': {'case_id': case_id,
                'source': {'kind': 'known', 'value': {'family': 'resolution', 'id': source['id'], 'revision': source['revision']}},
                'qualification': None}, 'calendar': None, 'ordered_quantity': None,
                'qualification': {'statement': 'Declared applicability', 'locator': 'Resolution page 1',
                    'scope_applies': {'kind': 'known', 'value': True},
                    'unresolved_incident': {'kind': 'known', 'value': False},
                    'conditions': [{'id': c['id'], 'applies': {'kind': 'known', 'value': True},
                        'locator': 'Declared condition'} for c in profile_row['definition']['conditions']]}}}}}


def correct(row):
    return {'operation_id': str(uuid4()), 'deadline_id': row['id'], 'change': {
        'action': 'correct', 'expected_revision': row['revision'],
        'definition': copy.deepcopy(row['definition']), 'reason': 'Correct declared tracking',
        'tracking': copy.deepcopy(row['tracking']['policies'])}}


def prepare(route, command, **kwargs):
    return request('POST', route + '/prepare', command, **kwargs)


def submit(route, draft, **kwargs):
    command = draft['command']
    action = command['change']['action']
    path = route if action == 'register' else route + '/' + command['deadline_id']
    if action == 'set_attention':
        path += '/attention'
    if action == 'retire':
        path += '/retirement'
    kwargs.setdefault('expected', 201)
    return request('PUT' if action == 'correct' else 'POST', path,
        {'command': command, 'expected_submission_digest': draft['submission_digest']}, **kwargs)


def stable_read(value):
    """Preserve returned evidence and freshness while normalizing a new read instant."""
    if isinstance(value, list):
        return [stable_read(item) for item in value]
    if not isinstance(value, dict):
        return value
    out = {key: stable_read(item) for key, item in value.items()}
    if 'operational' in out:
        operational = out['operational']
        checked = operational['checked_at']
        if operational['freshness'] == 'not_checked':
            assert checked is None
        else:
            assert isinstance(checked, dict)
            assert set(checked) == {'unix_seconds', 'nanosecond', 'offset_seconds'}
            assert isinstance(checked['unix_seconds'], int) and checked['unix_seconds'] > 0
            assert 0 <= checked['nanosecond'] < 1_000_000_000
            operational['checked_at'] = 'validated read instant'
    return out
