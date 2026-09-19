#!/usr/bin/env python3
"""Exercise durable alert generation, personal reads and preferences over HTTP."""
import copy
from datetime import datetime, timedelta, timezone
import json
import sys
import time
from urllib.parse import urlencode
from uuid import uuid4

from api_deadlines_support import STATE, TOKEN, request

ALERT_STATE = STATE.with_name('alerts-api-state.json')


def inbox(read='all', state='all'):
    parameters = {'limit': 100, 'read': read, 'state': state}
    rows, cursors = [], set()
    for _ in range(8):
        page = request('GET', '/api/v1/alerts?' + urlencode(parameters))
        assert page['checked_at']['offset_seconds'] == 0
        rows.extend(page['alerts'])
        assert len({row['id'] for row in rows}) == len(rows)
        if not page['has_more']:
            assert page['next_cursor'] is None
            return rows
        cursor = page['next_cursor']
        assert cursor and cursor not in cursors
        cursors.add(cursor)
        parameters['cursor'] = cursor
    raise AssertionError('Alert inbox exceeded the fixture scan budget')


def checkpoint_hearing_agenda(hearing_id):
    """Include only the new alert fixture in the existing global agenda snapshot."""
    path = STATE.with_name('hearing-api-state.json')
    saved = json.loads(path.read_text(encoding='utf-8'))
    agendas = [route for route in saved['records'] if route.startswith('/api/v1/hearings?')]
    assert len(agendas) == 1
    route = agendas[0]
    previous = saved['records'][route]
    current = request('GET', route)
    before = {row['id']: row for row in previous['hearings']}
    after = {row['id']: row for row in current['hearings']}
    assert all(after.get(identifier) == row for identifier, row in before.items())
    assert set(after) - set(before) <= {hearing_id}
    assert not previous['has_more'] and not current['has_more']
    saved['records'][route] = current
    path.write_text(json.dumps(saved, sort_keys=True), encoding='utf-8')


def capture():
    tokens = json.loads(STATE.read_text())['agenda']['tokens']
    actor = request('GET', '/api/v1/auth/me')['id']
    defaults = request('GET', '/api/v1/alert-preferences')['preferences']
    assert defaults['user_id'] == actor and defaults['revision'] == 0
    assert defaults['email_transport'] == 'disabled'
    assert defaults['values']['hearing_upcoming'] == {
        'lead_hours': [48, 24], 'channels': {'internal': True, 'email': True}}
    case = request('POST', '/api/v1/penal-cases', {
        'title': 'Synthetic personal alert', 'reference': 'ALERT-API',
        'profile': {'nuc': 'ALERT-NUC', 'nuc_authority': 'Declared authority',
                    'judicial_case_number': 'ALERT-CJ', 'judicial_authority': 'Declared court',
                    'offenses': ['Synthetic offense'], 'general_information': None,
                    'complementary_identifiers': None}}, 201)
    route = '/api/v1/cases/' + case['id']
    request('PUT', route + '/members/' + actor, expected=204)
    client = request('GET', '/api/v1/auth/me', token=tokens['client'])['id']
    request('PUT', route + '/members/' + client, expected=204)
    context = request('GET', route + '/hearings/context')
    scheduled = (datetime.now(timezone.utc) + timedelta(hours=36)).replace(microsecond=0)
    command = {'operation_id': str(uuid4()), 'hearing_id': str(uuid4()), 'change': {
        'action': 'schedule', 'expected_revision': 0,
        'expected_case_revision': context['case_revision'],
        'expected_stage_revision': context['stage_revision'],
        'values': {'kind': 'initial', 'scheduled_at': scheduled.isoformat(),
                   'modality': 'in_person', 'venue': 'Synthetic alert court', 'note': None,
                   'participants': [], 'conviction_basis': None}}}
    prepared = request('POST', route + '/hearings/prepare', command)
    hearing = request('POST', route + '/hearings', {
        'command': prepared['command'],
        'expected_submission_digest': prepared['submission_digest']}, 201)
    expires = time.monotonic() + 60
    while True:
        matches = [row for row in inbox(state='active') if row['subject']['id'] == hearing['id']]
        if matches:
            break
        assert time.monotonic() < expires, 'The live alert worker did not activate the hearing'
        time.sleep(0.25)
    assert len(matches) == 1
    alert = matches[0]
    assert alert['recipient_id'] == actor and alert['read_at'] is None
    assert alert['subject'] == {'kind': 'hearing', 'case_id': case['id'], 'id': hearing['id']}
    assert alert['case_title'] == case['administration']['title']
    assert alert['case_reference'] == case['administration']['reference']
    assert alert['origin'] == {'revision': 1, 'evidence_digest': hearing['receipt']['submission_digest']}
    assert alert['kind'] == {'kind': 'upcoming', 'lead_hours': 48, 'activity_at': {
        'unix_seconds': int(scheduled.timestamp()), 'nanosecond': 0, 'offset_seconds': 0}}
    assert alert['email'] == {'kind': 'disabled'}
    path = '/api/v1/alerts/' + alert['id']
    exact = route + '/hearings/' + hearing['id'] + '/revisions/1'
    assert request('GET', exact) == hearing
    assert request('GET', path)['alert']['read_at'] is None
    operation = {'operation_id': str(uuid4())}
    receipt = request('POST', path + '/read', operation)
    assert receipt['operation_id'] == operation['operation_id']
    assert receipt['alert']['read_at'] is not None
    assert request('POST', path + '/read', operation)['alert'] == receipt['alert']
    assert alert['id'] not in {row['id'] for row in inbox(read='unread')}
    values = copy.deepcopy(defaults['values'])
    values['deadline_upcoming']['lead_hours'] = [72, 24]
    change = {'operation_id': str(uuid4()), 'expected_revision': 0, 'values': values}
    saved = request('PUT', '/api/v1/alert-preferences', change)['preferences']
    assert saved['revision'] == 1 and saved['values'] == values
    assert saved['receipt'] == {'operation_id': change['operation_id'], 'expected_revision': 0}
    assert request('PUT', '/api/v1/alert-preferences', change)['preferences'] == saved
    assert request('GET', '/api/v1/alert-preferences')['preferences'] == saved
    assert request('GET', path)['alert'] == receipt['alert']
    assert alert['id'] in {row['id'] for row in inbox()}
    assert request('GET', exact) == hearing
    assert request('GET', route + '/hearings/' + hearing['id'] + '/history')['revisions'] == [hearing]
    for token, status, code in [('', 401, 'invalid_session'),
                                (tokens['client'], 403, 'permission_denied')]:
        for method, endpoint, body in [
            ('GET', '/api/v1/alerts', None), ('GET', path, None),
            ('POST', path + '/read', {'operation_id': str(uuid4())}),
            ('GET', '/api/v1/alert-preferences', None),
            ('PUT', '/api/v1/alert-preferences', change)]:
            request(method, endpoint, body, status, token=token, code=code)
    request('GET', path, token=tokens['paralegal'], expected=404, code='alert_not_found')
    ALERT_STATE.write_text(json.dumps({
        'preferences': saved, 'alert': receipt['alert'], 'hearing': hearing,
        'exact_path': exact, 'history_path': route + '/hearings/' + hearing['id'] + '/history',
    }, sort_keys=True), encoding='utf-8')
    ALERT_STATE.chmod(0o600)
    checkpoint_hearing_agenda(hearing['id'])
    assert request('GET', '/api/v1/audit/verify', token=TOKEN)['valid']
    print('Alert API passed: live worker, 48-hour hearing, personal inbox, idempotent reading, '
          'preferences, exact history, disabled email and denied anonymous/Client/foreign access.')


def restore():
    saved = json.loads(ALERT_STATE.read_text(encoding='utf-8'))
    alert = saved['alert']
    assert request('GET', '/api/v1/auth/me')['id'] == alert['recipient_id']
    assert request('GET', '/api/v1/alert-preferences')['preferences'] == saved['preferences']
    assert request('GET', '/api/v1/alerts/' + alert['id'])['alert'] == alert
    matching = [row for row in inbox() if row['subject']['id'] == saved['hearing']['id']]
    assert matching == [alert], 'Restored alert must preserve its identity and original read time'
    assert alert['id'] not in {row['id'] for row in inbox(read='unread')}
    assert request('GET', saved['exact_path']) == saved['hearing']
    assert request('GET', saved['history_path'])['revisions'] == [saved['hearing']]
    assert request('GET', '/api/v1/audit/verify', token=TOKEN)['valid']
    print('Alert restore passed: exact preferences, read alert, recipient, captured origin '
          'and hearing history preserved without a duplicate occurrence.')


if __name__ == '__main__':
    {'capture': capture, 'restore': restore}[sys.argv[1]]()
