"""Observe exact own-hearing agenda entries and real personal alert activation."""
from datetime import datetime, timedelta
import time
from urllib.parse import urlencode
from uuid import uuid4

from api_resource_hearings_support import TOKEN, request, route


def scheduled(result):
    return datetime.fromisoformat(result['hearing']['values']['scheduled_at'].replace('Z', '+00:00'))


def instant(result):
    return {'unix_seconds': int(scheduled(result).timestamp()), 'nanosecond': 0, 'offset_seconds': 0}


def period(results):
    start = min(map(scheduled, results)) - timedelta(hours=1)
    until = max(map(scheduled, results)) + timedelta(hours=1)
    return {'from': start.isoformat().replace('+00:00', 'Z'),
            'until': until.isoformat().replace('+00:00', 'Z'), 'limit': 100,
            'kind': 'resource_hearing', 'hearing_status': 'all'}


def agenda_page(params, token=TOKEN):
    value = request('GET', '/api/v1/agenda?' + urlencode(params), token=token)
    assert value['complete'] and value['next_cursor'] is None
    assert value['checked_at']['offset_seconds'] == 0
    return value['items']


def agenda(results, status='active', token=TOKEN):
    params = period(results)
    rows = agenda_page(params, token)
    expected = sorted(results, key=lambda value: (scheduled(value), value['hearing']['id']))
    selected = [row for row in rows
                if row['kind'] == 'resource_hearing'
                and row['resource_hearing']['case_id'] == results[0]['hearing']['case_id']]
    assert len(selected) == len(expected)
    for row, result in zip(selected, expected):
        hearing, values = result['hearing'], result['hearing']['values']
        assert row['at'] == instant(result)
        assert row['resource_hearing'] == {
            'case_id': hearing['case_id'], 'resource_id': hearing['resource_id'],
            'id': hearing['id'], 'revision': 1, 'kind': values['kind'],
            'scheduled_at': values['scheduled_at'], 'modality': values['modality'],
            'participant_count': len(values['participants']),
            'association_id': hearing['association_id'], 'capture_digest': hearing['capture_digest'],
        }
        assert row['case_status'] == status
    assert agenda_page({**params, 'hearing_status': 'scheduled'}, token) == rows
    request('GET', '/api/v1/agenda?' + urlencode({**params, 'hearing_status': 'cancelled'}),
            expected=400, token=token)
    all_rows = agenda_page({**params, 'kind': 'all'}, token)
    assert [row for row in all_rows if row['kind'] == 'resource_hearing'] == rows
    cancelled = agenda_page({**params, 'kind': 'all', 'hearing_status': 'cancelled'}, token)
    assert all(row['kind'] != 'resource_hearing' for row in cancelled)
    return selected


def inbox(token=TOKEN):
    rows, seen, cursor = [], set(), None
    for _ in range(8):
        params = {'limit': 100, 'read': 'all', 'state': 'all'}
        if cursor:
            params['cursor'] = cursor
        page = request('GET', '/api/v1/alerts?' + urlencode(params), token=token)
        rows.extend(page['alerts'])
        assert len({row['id'] for row in rows}) == len(rows)
        if not page['has_more']:
            assert page['next_cursor'] is None
            return rows
        cursor = page['next_cursor']
        assert cursor and cursor not in seen
        seen.add(cursor)
    raise AssertionError('Own hearing inbox exceeded its bounded page budget')


def own_alerts(result, token=TOKEN):
    hearing = result['hearing']
    return [row for row in inbox(token) if row['subject']['kind'] == 'resource_hearing'
            and row['subject']['id'] == hearing['id']]


def upcoming(result, actor, token=TOKEN):
    expires = time.monotonic() + 60
    while True:
        rows = own_alerts(result, token)
        if rows:
            break
        assert time.monotonic() < expires, 'The live worker did not emit the own hearing alert'
        time.sleep(0.25)
    assert len(rows) == 1
    row, hearing = rows[0], result['hearing']
    assert row['recipient_id'] == actor
    assert row['subject'] == {'kind': 'resource_hearing', 'case_id': hearing['case_id'],
                             'resource_id': hearing['resource_id'], 'id': hearing['id']}
    assert row['origin'] == {'revision': 1, 'evidence_digest': hearing['capture_digest']}
    assert row['kind'] == {'kind': 'upcoming', 'lead_hours': 48, 'activity_at': instant(result)}
    assert row['state'] == {'kind': 'active'}
    assert row['email'] == {'kind': 'disabled'}
    assert request('GET', '/api/v1/alerts/' + row['id'], token=token)['alert'] == row
    return row


def read(result, actor, token=TOKEN):
    alert = upcoming(result, actor, token)
    path = '/api/v1/alerts/' + alert['id'] + '/read'
    operation = {'operation_id': str(uuid4())}
    response = request('POST', path, operation, token=token)
    assert response['operation_id'] == operation['operation_id']
    assert response['alert']['read_at'] is not None
    assert request('POST', path, operation, token=token)['alert'] == response['alert']
    return {'command': operation, 'alert': response['alert']}


def verify_read(result, saved, token=TOKEN):
    row = saved['alert']
    path = '/api/v1/alerts/' + row['id']
    assert own_alerts(result, token) == [row]
    assert request('GET', path, token=token)['alert'] == row
    assert request('POST', path + '/read', saved['command'], token=token)['alert'] == row
    unread = request('GET', '/api/v1/alerts?limit=100&read=unread&state=all', token=token)
    assert row['id'] not in {item['id'] for item in unread['alerts']}


def denied_alerts(scenario, result, alert):
    accounts = scenario['accounts']
    path = '/api/v1/alerts/' + alert['id']
    for token, status, code in [('', 401, 'invalid_session'),
                                (accounts['client']['token'], 403, 'permission_denied')]:
        for method, endpoint, body in [('GET', '/api/v1/alerts', None), ('GET', path, None),
                                      ('POST', path + '/read', {'operation_id': str(uuid4())})]:
            request(method, endpoint, body, status, token, code)
    outside = accounts['outside']['token']
    assert own_alerts(result, outside) == []
    request('GET', path, expected=404, token=outside, code='alert_not_found')
    assert agenda_page(period([result]), outside) == []
