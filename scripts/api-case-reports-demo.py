#!/usr/bin/env python3
"""Exercise report workers through HTTP and the disposable database restore."""
import csv
from datetime import datetime, timedelta, timezone
import hashlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import time
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen
from uuid import UUID, uuid4
from api_procedural_facts_support import BASE, TOKEN, WORK, request

ROOT = '/api/v1/case-reports'
STATE = WORK / 'case-reports-api-state.json'
PASSWORD = 'synthetic report restore fixture password'
HEADER = ('row_type,report_id,snapshot_digest,checked_at,requester_id,scope,created_from,'
          'created_before,status_filter,assigned_litigator_filter,case_id,title,reference,'
          'created_at,status,administration_revision,administration_digest,assigned_litigators,'
          'litigator_id,litigator_email,active_cases,closed_cases,total_cases').split(',')


def private(path, value):
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), 'wb') as stream:
        stream.write(value)


def exchange(method, path, body=None, token=TOKEN, expected=(200,), timeout=30):
    headers = {'Authorization': 'Bearer ' + token}
    if body is not None:
        headers['Content-Type'] = 'application/json'
        body = json.dumps(body, ensure_ascii=True).encode('ascii')
    try:
        response = urlopen(Request(BASE + path, data=body, headers=headers, method=method), timeout=timeout)
    except HTTPError as error:
        response = error
    with response:
        raw = response.read(16 * 1024 * 1024 + 1)
        assert len(raw) <= 16 * 1024 * 1024, 'Report HTTP response exceeds artifact budget'
        assert response.status in expected, (method, path, response.status, expected)
        return raw, {name.lower(): value for name, value in response.headers.items()}


def report(method, path, body=None, token=TOKEN, expected=(200,), timeout=30):
    raw, _ = exchange(method, path, body, token, expected, timeout)
    assert len(raw) <= 128 * 1024, 'Report metadata exceeds fixture budget'
    return json.loads(raw)


def pages(token=TOKEN, unread=False):
    rows, query = [], {'limit': 1, 'unread_only': str(unread).lower()}
    for _ in range(20):
        page = report('GET', ROOT + '?' + urlencode(query), token=token)
        assert len(page['reports']) <= 1
        rows.extend(page['reports'])
        assert [row['id'] for row in rows] == sorted({row['id'] for row in rows})
        if not page['has_more']:
            assert page['next_after_id'] is None
            return {row['id']: row for row in rows}
        assert page['reports'] and page['next_after_id'] == rows[-1]['id']
        query['after_id'] = page['next_after_id']
    raise AssertionError('Report fixture exceeded its page budget')


def login(enrollment, index):
    challenge = request('POST', '/api/v1/auth/login', {
        'email': enrollment['user']['email'], 'password': PASSWORD,
    })
    return request('POST', '/api/v1/auth/mfa/recovery', {
        'challenge_token': challenge['challenge_token'], 'code': enrollment['recovery_codes'][index],
    })['access_token']


def instant(value):
    return datetime.fromisoformat(value.replace('Z', '+00:00'))


def ready(command, token):
    initial = report('POST', ROOT, command, token, (202,))
    assert initial['state'] == 'queued' and initial['notice'] is None
    assert initial['operation_id'] == command['operation_id']
    identifier = str(UUID(initial['id']))
    replay = report('POST', ROOT, command, token, (200, 202))
    assert replay['id'] == identifier and replay['request_digest'] == initial['request_digest']
    observed, deadline = [('queued', None)], time.monotonic() + 90
    order = {('queued', None): 0, ('processing', 'capturing'): 1,
             ('processing', 'rendering'): 2, ('ready', None): 3}
    while time.monotonic() < deadline:
        value = report('GET', ROOT + '/' + identifier, token=token,
                       timeout=min(10, max(0.1, deadline - time.monotonic())))
        state = (value['state'], value['phase'])
        assert value['failure'] is None, (identifier, value['state'], value['failure'])
        assert state in order and order[state] >= order[observed[-1]], (observed, state)
        if state != observed[-1]:
            observed.append(state)
        assert value['state'] in ['queued', 'processing', 'ready'], observed
        if value['state'] == 'processing':
            assert value['phase'] in ['capturing', 'rendering']
        if value['state'] == 'ready':
            assert value['phase'] is None and value['retry_at'] is None
            assert value['notice']['kind'] == 'ready' and value['notice']['read_at'] is None
            assert report('POST', ROOT, command, token) == value
            return value, observed
        assert value['ready'] is None and value['notice'] is None
        time.sleep(min(0.25, max(0, deadline - time.monotonic())))
    raise AssertionError(('Composed server did not publish both report artifacts within 90s', observed))


def pair(saved, token, capture=False):
    detail = saved['detail']
    metadata = {row['format']: row for row in detail['ready']['artifacts']}
    assert len(detail['ready']['artifacts']) == 2 and set(metadata) == {'pdf', 'csv'}
    blobs = {}
    for format_name, media in [('pdf', 'application/pdf'), ('csv', 'text/csv; charset=utf-8')]:
        path = WORK / ('report-' + detail['id'] + '.' + format_name)
        raw, headers = exchange('GET', ROOT + '/' + detail['id'] + '/download?format=' + format_name, token=token)
        assert headers['content-type'] == media and headers['cache-control'] == 'no-store'
        assert headers['x-content-type-options'] == 'nosniff'
        assert headers['content-disposition'] == 'attachment; filename="' + path.name + '"'
        assert headers['x-report-id'] == detail['id']
        assert headers['x-report-snapshot-digest'] == detail['ready']['snapshot_digest']
        assert len(raw) == int(headers['content-length']) == metadata[format_name]['bytes']
        assert hashlib.sha256(raw).hexdigest() == headers['x-report-digest'] == metadata[format_name]['digest']
        if capture:
            private(path, raw)
        else:
            assert raw == path.read_bytes(), ('Stored report bytes changed', detail['id'], format_name)
        blobs[format_name] = raw
    assert report('GET', ROOT + '/' + detail['id'], token=token) == detail, 'Download marked notice read'
    identities(saved, blobs)


def identities(saved, blobs):
    detail, cases = saved['detail'], saved['cases']
    reader = csv.DictReader(io.StringIO(blobs['csv'].decode('utf-8'), newline=''))
    assert reader.fieldnames == HEADER
    rows = list(reader)
    assert len(rows) == len(cases) + 2 and rows[0]['row_type'] == 'capture'
    for row in rows:
        assert set(row) == set(HEADER) and None not in row.values()
        assert row['report_id'] == detail['id'] and row['snapshot_digest'] == detail['ready']['snapshot_digest']
        assert instant(row['checked_at']) == instant(detail['ready']['checked_at'])
        assert row['requester_id'] == saved['requester_id'] and row['scope'] == detail['scope']
        for key in ['created_from', 'created_before']:
            assert instant(row[key]) == instant(detail['filters'][key])
        assert row['status_filter'] == detail['filters']['status']
        assert row['assigned_litigator_filter'] == detail['filters']['assigned_litigator']
    captured = rows[1:-1]
    assert [row['case_id'] for row in captured] == sorted(cases)
    for row in captured:
        case = cases[row['case_id']]
        assert row['row_type'] == 'case'
        assert row['title'] == "'" + case['title'] and row['reference'] == "'" + case['reference']
        assert row['status'] == case['administrative_status']
        assert row['administration_revision'] == (str(case['revision']) if case['revision'] else '')
        assert row['administration_digest'] == (case['values_digest'] or '')
        assert row['assigned_litigators'].startswith("'")
        assert json.loads(row['assigned_litigators'][1:]) == [saved['litigator']]
        assert instant(row['created_from']) <= instant(row['created_at']) < instant(row['created_before'])
    workload = rows[-1]
    assert workload['row_type'] == 'workload' and workload['litigator_id'] == saved['litigator']['user_id']
    assert workload['litigator_email'] == "'" + saved['litigator']['email']
    active = sum(case['administrative_status'] == 'active' for case in cases.values())
    for row in [rows[0], workload]:
        assert [int(row[key]) for key in ['active_cases', 'closed_cases', 'total_cases']] == \
            [active, len(cases) - active, len(cases)]
    assert blobs['pdf'].startswith(b'%PDF-')
    pdf = WORK / ('report-' + detail['id'] + '.pdf')
    text = subprocess.run(['pdftotext', '-enc', 'UTF-8', '-layout', str(pdf), '-'],
                          check=True, capture_output=True, text=True, timeout=15).stdout
    compact = re.sub(r'\s+', '', text)
    for value in [detail['id'], detail['ready']['snapshot_digest'], saved['requester_id'], *cases]:
        assert value in compact, ('Missing captured PDF identity', value)
    for identifier, case in cases.items():
        assert compact.count(identifier) == 1
        assert re.sub(r'\s+', '', case['title']) in compact
        assert re.sub(r'\s+', '', case['reference']) in compact


def access_denied(identifier, token, expected, code):
    for method, suffix, body in [('GET', '', None), ('POST', '/notice-read', {}),
                                 ('GET', '/download?format=pdf', None), ('GET', '/download?format=csv', None)]:
        denied = report(method, ROOT + '/' + identifier + suffix, body, token, (expected,))
        assert denied['error']['code'] == code


def roles_denied(saved):
    tokens = json.loads((WORK / 'deadlines-api-state.json').read_text())['agenda']['tokens']
    for role in ['client', 'paralegal']:
        for method, path, body in [('GET', ROOT, None), ('POST', ROOT, saved['command'])]:
            denied = report(method, path, body, tokens[role], (403,))
            assert denied['error']['code'] == 'permission_denied'
        access_denied(saved['detail']['id'], tokens[role], 403, 'permission_denied')


def capture():
    enrollments = {role: request('POST', '/api/v1/users', {
        'email': 'report-restore-' + role + '@example.com', 'password': PASSWORD, 'role': role,
    }, 201) for role in ['litigator', 'owner']}
    tokens = {role: login(row, 0) for role, row in enrollments.items()}
    litigator = {'user_id': enrollments['litigator']['user']['id'],
                'email': enrollments['litigator']['user']['email']}
    cases = {}
    for status in ['active', 'closed']:
        case = request('POST', '/api/v1/cases', {
            'title': 'Report captured ' + status, 'reference': 'REPORT-' + status.upper(),
        }, 201)
        path = '/api/v1/cases/' + case['id']
        request('PUT', path + '/members/' + litigator['user_id'], expected=204)
        admin = request('GET', path + '/administration')['administration']
        if status == 'closed':
            admin = request('PUT', path + '/administrative-status', {
                'expected_revision': admin['revision'], 'administrative_status': 'closed',
            })['administration']
        cases[case['id']] = admin
    now = datetime.now(timezone.utc)
    filters = {'created_from': (now - timedelta(hours=1)).isoformat(),
               'created_before': (now + timedelta(hours=1)).isoformat(),
               'status': 'all', 'assigned_litigator': litigator['user_id']}
    owner_id = request('GET', '/api/v1/auth/me')['id']
    reports = {}
    for label, token, status, requester in [('unread', TOKEN, 'all', owner_id),
                                           ('read', TOKEN, 'active', owner_id),
                                           ('revoked', tokens['litigator'], 'all', litigator['user_id'])]:
        command = {'operation_id': str(uuid4()), 'filters': {**filters, 'status': status}}
        detail, observed = ready(command, token)
        assert detail['scope'] == ('assigned_cases' if label == 'revoked' else 'office')
        saved = {'command': command, 'detail': detail, 'requester_id': requester, 'states': observed,
                 'litigator': litigator, 'cases': {key: case for key, case in cases.items()
                                                if status == 'all' or case['administrative_status'] == status}}
        pair(saved, token, capture=True)
        reports[label] = saved
    for saved in reports.values():
        access_denied(saved['detail']['id'], tokens['owner'], 404, 'case_report_not_found')
    assert pages(tokens['owner']) == {}
    read_id, unread_id = reports['read']['detail']['id'], reports['unread']['detail']['id']
    assert {read_id, unread_id} <= pages(unread=True).keys()
    acknowledged = report('POST', ROOT + '/' + read_id + '/notice-read', {})
    assert acknowledged['notice']['read_at'] is not None
    assert report('POST', ROOT + '/' + read_id + '/notice-read', {}) == acknowledged
    reports['read']['detail'] = acknowledged
    assert read_id not in pages(unread=True) and unread_id in pages(unread=True)
    conflict = {**reports['unread']['command'], 'filters': {**filters, 'status': 'closed'}}
    assert report('POST', ROOT, conflict, expected=(409,))['error']['code'] == 'case_report_operation_conflict'
    closed = next(case for case in cases.values() if case['administrative_status'] == 'closed')
    request('PUT', '/api/v1/cases/' + closed['case_id'] + '/administrative-status', {
        'expected_revision': closed['revision'], 'administrative_status': 'active',
    })
    pair(reports['unread'], TOKEN)
    assert set(pages(tokens['litigator'])) == {reports['revoked']['detail']['id']}
    request('DELETE', '/api/v1/cases/' + closed['case_id'] + '/members/' + litigator['user_id'], expected=204)
    retained = next(key for key in cases if key != closed['case_id'])
    request('GET', '/api/v1/cases/' + retained, token=tokens['litigator'])
    access_denied(reports['revoked']['detail']['id'], tokens['litigator'], 403, 'case_report_access_revoked')
    assert pages(tokens['litigator']) == {}
    roles_denied(reports['unread'])
    private(STATE, json.dumps({'enrollments': enrollments, 'tokens': tokens, 'reports': reports,
                               'retained_case': retained}, sort_keys=True).encode('ascii'))
    print('Report worker HTTP states: ' + json.dumps({key: row['states'] for key, row in reports.items()}))
    print('Report API passed: queued HTTP jobs, paired captured artifacts, own notices and whole-report revocation.')


def restore():
    saved = json.loads(STATE.read_text())
    for token in saved['tokens'].values():
        request('GET', '/api/v1/auth/me', token=token, expected=401, code='invalid_session')
    tokens = {role: login(row, 1) for role, row in saved['enrollments'].items()}
    reports = saved['reports']
    for label in ['unread', 'read']:
        row = reports[label]
        assert report('GET', ROOT + '/' + row['detail']['id']) == row['detail']
        assert report('POST', ROOT, row['command']) == row['detail']
        pair(row, TOKEN)
        access_denied(row['detail']['id'], tokens['owner'], 404, 'case_report_not_found')
    unread_id, read_id = reports['unread']['detail']['id'], reports['read']['detail']['id']
    assert unread_id in pages(unread=True) and read_id not in pages(unread=True)
    assert report('POST', ROOT + '/' + read_id + '/notice-read', {}) == reports['read']['detail']
    request('GET', '/api/v1/cases/' + saved['retained_case'], token=tokens['litigator'])
    access_denied(reports['revoked']['detail']['id'], tokens['litigator'], 403, 'case_report_access_revoked')
    assert pages(tokens['litigator']) == pages(tokens['owner']) == {}
    roles_denied(reports['unread'])
    print('Report restore passed: exact PDF/CSV bytes, identities, read/unread notices and revoked full capture.')


if __name__ == '__main__':
    assert len(sys.argv) == 2 and sys.argv[1] in ['capture', 'restore']
    assert shutil.which('pdftotext'), 'Report acceptance requires Poppler pdftotext'
    capture() if sys.argv[1] == 'capture' else restore()
