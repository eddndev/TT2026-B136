"""Observe native precautionary agenda, alert origin and retained exact reads."""
from datetime import datetime, timedelta, timezone
import time
from urllib.parse import urlencode
from uuid import uuid4

from api_resource_hearings_observation import inbox
from api_precautionary_fixtures import (
    TOKEN, actor_token, collection, payload, request, review, route,
)


def values(operation):
    return operation['capture']['review']['resolved_values']


def scheduled(operation):
    return datetime.fromisoformat(values(operation)['scheduled_at'].replace('Z', '+00:00'))


def instant(operation):
    return {'unix_seconds': int(scheduled(operation).timestamp()), 'nanosecond': 0,
            'offset_seconds': 0}


def agenda(scenario, hearings, token=TOKEN):
    start = min(map(scheduled, hearings)).astimezone(timezone.utc) - timedelta(hours=1)
    until = max(map(scheduled, hearings)).astimezone(timezone.utc) + timedelta(hours=1)
    params = {'from': start.isoformat().replace('+00:00', 'Z'),
              'until': until.isoformat().replace('+00:00', 'Z'),
              'kind': 'precautionary_hearing', 'hearing_status': 'all', 'limit': 100}

    def rows(status):
        page = request('GET', '/api/v1/agenda?' + urlencode({**params, 'hearing_status': status}),
                       token=token)
        assert page['complete'] and page['next_cursor'] is None
        return [row for row in page['items'] if row['kind'] == 'precautionary_hearing'
                and row['precautionary_hearing']['case_id'] == scenario['case_id']]

    all_rows = rows('all')
    expected = sorted(hearings, key=lambda item: (
        scheduled(item), item['capture']['review']['command']['hearing_id']))
    assert len(all_rows) == len(expected)
    for row, operation in zip(all_rows, expected):
        capture = operation['capture']
        reviewed = capture['review']
        value = reviewed['resolved_values']
        assert row['at'] == instant(operation) and row['case_status'] == 'active'
        assert row['precautionary_hearing'] == {
            'case_id': scenario['case_id'], 'id': reviewed['command']['hearing_id'],
            'revision': reviewed['result_revision'], 'purpose': value['purpose'],
            'scheduled_at': value['scheduled_at'], 'modality': value['modality'],
            'status': reviewed['status'], 'participant_count': len(value['participants']),
            'capture_digest': capture['capture_digest']}
    for status in ['scheduled', 'cancelled']:
        assert rows(status) == [row for row in all_rows
                                if row['precautionary_hearing']['status'] == status]
    return all_rows


def hearing_alerts(operation, token):
    reviewed = operation['capture']['review']
    return [row for row in inbox(token) if row['subject'] == {
        'kind': 'precautionary_hearing', 'case_id': reviewed['case_id'],
        'id': reviewed['command']['hearing_id']}]


def await_alert(operation, token, resolved=False):
    expires = time.monotonic() + 60
    while True:
        rows = hearing_alerts(operation, token)
        if len(rows) == 1 and (rows[0]['state']['kind'] == 'resolved') == resolved:
            return rows[0]
        assert time.monotonic() < expires, 'Native precautionary alert did not reach expected state'
        time.sleep(0.25)


def upcoming(operation, account):
    alert = await_alert(operation, account['token'])
    capture = operation['capture']
    assert alert['recipient_id'] == account['id']
    assert alert['origin'] == {'revision': capture['review']['result_revision'],
                               'evidence_digest': capture['capture_digest']}
    assert alert['kind'] == {'kind': 'upcoming', 'lead_hours': 48, 'activity_at': instant(operation)}
    assert alert['state'] == {'kind': 'active'}
    assert alert['email'] == {'kind': 'disabled'}
    assert request('GET', '/api/v1/alerts/' + alert['id'], token=account['token'])['alert'] == alert
    return alert


def mark_read(operation, account):
    alert = upcoming(operation, account)
    body = {'operation_id': str(uuid4())}
    path = '/api/v1/alerts/' + alert['id'] + '/read'
    result = request('POST', path, body, token=account['token'])
    assert result['operation_id'] == body['operation_id'] and result['alert']['read_at'] is not None
    repeated = request('POST', path, body, token=account['token'])
    assert repeated['operation_id'] == result['operation_id']
    assert repeated['alert'] == result['alert']
    return {'command': body, 'alert': result['alert']}


def verify_alerts(scenario, hearings, saved):
    token = scenario['accounts']['litigator']['token']
    original, cancelled = saved['read']['alert'], saved['resolved']
    assert hearing_alerts(hearings[0], token) == [original]
    assert hearing_alerts(hearings[1], token) == [cancelled]
    path = '/api/v1/alerts/' + original['id']
    assert request('GET', path, token=token)['alert'] == original
    assert request('POST', path + '/read', saved['read']['command'], token=token)['alert'] == original
    assert cancelled['state']['kind'] == 'resolved'
    assert original['occurrence_id'] != cancelled['occurrence_id']


def exact_hearing_path(scenario, operation):
    reviewed = operation['capture']['review']
    return collection(scenario, 'hearing') + '/' + reviewed['command']['hearing_id'] + '/revisions/' + str(
        reviewed['result_revision'])


def exact_measure_path(scenario, record):
    reference = record['reference']
    return route(scenario['case_id']) + '/measures/' + reference['id'] + '/revisions/' + str(
        reference['revision']) + '?' + urlencode({'capture_digest': reference['capture_digest']})


def paged_records(scenario, paths):
    specifications = [
        (collection(scenario, 'hearing'), 'after_id',
         lambda item: item['capture']['review']['command']['hearing_id'], 2),
        (collection(scenario, 'decision'), 'after_id', lambda item: item['origin']['decision_id'], 1),
        (collection(scenario, 'administrative'), 'after_operation_id',
         lambda item: item['origin']['operation_id'], 2),
        (route(scenario['case_id']) + '/measures', 'after_id', lambda item: item['reference']['id'], 2),
    ]
    for base, cursor_name, identity, count in specifications:
        current = base + '?limit=20'
        page = request('GET', current)
        assert page['case_id'] == scenario['case_id']
        assert len(page['items']) == count and not page['has_more']
        assert page['next_' + cursor_name] is None
        assert list(map(identity, page['items'])) == sorted(map(identity, page['items']))
        paths[current] = page
        first = request('GET', base + '?limit=1')
        assert first['items'] == page['items'][:1] and first['has_more'] == (count > 1)
        if count > 1:
            cursor = first['next_' + cursor_name]
            assert cursor == identity(first['items'][0])
            tail = request('GET', base + '?limit=1&' + cursor_name + '=' + cursor)
            assert tail['items'] == page['items'][1:] and not tail['has_more']
            assert tail['next_' + cursor_name] is None


def permissions(scenario, operations, selected_paths, alert):
    accounts = scenario['accounts']
    for path in selected_paths:
        expected = request('GET', path)
        assert request('GET', path, token=accounts['paralegal']['token']) == expected
        request('GET', path, expected=403, token=accounts['client']['token'], code='permission_denied')
        request('GET', path, expected=404, token=accounts['outside']['token'], code='case_not_found')
    for item in operations:
        value = review(item['prepared'], item['kind'])
        path = collection(scenario, item['kind'])
        request('POST', path + '/prepare', value['command'], expected=403,
                token=accounts['paralegal']['token'], code='permission_denied')
        request('POST', path + '/submit', payload(item['prepared'], item['kind']), expected=403,
                token=accounts['paralegal']['token'], code='permission_denied')
    path = '/api/v1/alerts/' + alert['id']
    request('GET', path, expected=403, token=accounts['client']['token'], code='permission_denied')
    request('GET', path, expected=404, token=accounts['outside']['token'], code='alert_not_found')
