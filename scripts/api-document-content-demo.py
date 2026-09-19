#!/usr/bin/env python3
"""Accept exact unsealed content and durable Owner incidents, then restore them."""
import base64
import hashlib
import json
import sys
from uuid import uuid4
from api_document_content_support import (
    INBOX, STATE, TOKEN, content, content_path, corrupted_ciphertext,
    counts, enroll, incident_pages, request,
)


def new_case(label):
    case = request('POST', '/api/v1/cases', {
        'title': 'Exact content ' + label, 'reference': 'CONTENT-' + label,
    }, 201)
    return '/api/v1/cases/' + case['id']


def permissions(route, first, payload, users, tokens):
    path = content_path(route, first)
    for token in [TOKEN, tokens['litigator'], tokens['paralegal']]:
        content(path, first, payload, token)
    before = counts(first)
    content(path, first, payload, tokens['client'], 403, 'permission_denied')
    foreign = new_case('foreign')
    content(content_path(foreign, first), first, payload, TOKEN, 404, 'document_not_found')
    hidden = content(content_path(foreign, first), first, payload,
                     tokens['litigator'], 404, 'document_not_found')
    missing_route = '/api/v1/cases/' + str(uuid4())
    missing = content(content_path(missing_route, first), first, payload,
                      tokens['litigator'], 404, 'document_not_found')
    assert hidden == missing, 'Foreign and absent cases must not disclose different content'
    request('DELETE', route + '/members/' + users['litigator'], expected=204)
    assert content(path, first, payload, tokens['litigator'], 404, 'document_not_found') == missing
    assert counts(first) == before, 'Denied reads must not authorize content or create incidents'
    request('PUT', route + '/members/' + users['litigator'], expected=204)
    admin = request('GET', route + '/administration')['administration']
    request('PUT', route + '/administrative-status', {
        'expected_revision': admin['revision'], 'administrative_status': 'closed',
    })
    for token in [TOKEN, tokens['litigator'], tokens['paralegal']]:
        content(path, first, payload, token)
    assert request('GET', route + '/administration')['administration']['administrative_status'] == 'closed'


def rejection(route, first, payload, users, tokens):
    path = content_path(route, first)
    before = counts(first)
    with corrupted_ciphertext(first):
        # Unauthorized requests neither inspect corrupt bytes nor open incidents.
        content(path, first, payload, tokens['client'], 403, 'permission_denied')
        assert counts(first) == before
        content(path, first, payload, tokens['paralegal'], 409, 'document_content_validation_failed')
        after = counts(first)
        assert after == {**before, 'rejected': before['rejected'] + 1,
                         'incidents': before['incidents'] + 1}
    incident = next(row for row in incident_pages()
                    if row['case_id'] == first['case_id'] and row['document_id'] == first['id'])
    assert incident['document_version'] == 1 and incident['requester_id'] == users['paralegal']
    assert incident['failure'] == 'authentication_failed'
    assert incident['expected_digest'] == first['digest']
    assert len(bytes.fromhex(incident['observed_snapshot_digest'])) == 32
    assert set(incident) == {
        'id', 'observation_id', 'case_id', 'document_id', 'document_version', 'requester_id',
        'failure', 'detected_at', 'recorded_at', 'expected_digest', 'observed_snapshot_digest',
    }, 'The Owner incident must not expose plaintext, vault, keys or tokens'
    detail_path = INBOX + '/' + incident['id']
    assert request('GET', detail_path) == incident
    for token in tokens.values():
        request('GET', INBOX, expected=403, token=token, code='permission_denied')
        denial = request('GET', detail_path, expected=403, token=token, code='permission_denied')
        assert request('GET', INBOX + '/' + str(uuid4()), expected=403,
                       token=token, code='permission_denied') == denial
    content(path, first, payload)
    assert request('GET', detail_path) == incident, 'Restoring fixture bytes must not erase its observation'
    return detail_path, incident


def capture():
    route = new_case('history')
    users, tokens = {}, {}
    for role in ['litigator', 'paralegal', 'client']:
        users[role], tokens[role] = enroll(role, route)
    payloads = [b'Original unsealed content\x00\xff\n', b'Second unsealed content\x01\xfe\n']
    first = request('POST', route + '/documents', payloads[0], 201,
                    headers={'X-Document-Name': 'unsealed-content.bin'})
    assert first['version'] == 1 and first['sealed'] is False
    before = counts(first)
    content(content_path(route, first), first, payloads[0])
    assert counts(first) == {**before, 'authorized': before['authorized'] + 1}
    document_path = route + '/documents/' + first['id']
    second = request('POST', document_path + '/versions?expected_version=1', payloads[1], 201,
                     headers={'X-Document-Name': 'unsealed-content-v2.bin'})
    assert second['version'] == 2 and second['sealed'] is False and second['id'] == first['id']
    assert request('GET', document_path) == second
    for row, payload in zip([first, second], payloads):
        assert row['digest'] == hashlib.sha256(payload).hexdigest()
        content(content_path(route, row), row, payload)
    permissions(route, first, payloads[0], users, tokens)
    incident_path, incident = rejection(route, first, payloads[0], users, tokens)
    snapshots = {incident_path: incident}
    for row in [first, second]:
        path = document_path + '/versions/' + str(row['version'])
        snapshots[path] = request('GET', path)
        exact = {key: value for key, value in row.items() if key != 'current_metadata'}
        assert snapshots[path] == exact and row['sealed'] is False
    snapshots[document_path + '/versions'] = request('GET', document_path + '/versions')
    snapshots[INBOX + '?limit=100'] = request('GET', INBOX + '?limit=100')
    request('GET', INBOX + '?limit=0', expected=400, code='invalid_incident_query')
    request('GET', INBOX + '?limit=101', expected=400, code='invalid_incident_query')
    request('POST', '/api/v1/auth/logout', expected=204, token=tokens['litigator'])
    before = counts(first)
    content(content_path(route, first), first, payloads[0], tokens['litigator'], 401, 'invalid_session')
    assert counts(first) == before
    assert request('GET', '/api/v1/audit/verify')['valid'] is True
    STATE.write_text(json.dumps({
        'route': route, 'rows': [first, second], 'snapshots': snapshots,
        'payloads': [base64.b64encode(value).decode('ascii') for value in payloads],
    }, ensure_ascii=True, sort_keys=True))
    print('Content API passed: unsealed V1/V2, exact history, roles, closure, revocation and audited rejection.')
    print('Disposable corruption was reversed byte-for-byte; the Owner incident remains historical.')


def restore():
    saved = json.loads(STATE.read_text())
    for path, response in saved['snapshots'].items():
        assert request('GET', path) == response, ('Restored content/incident response changed', path)
    for row, encoded in zip(saved['rows'], saved['payloads']):
        content(content_path(saved['route'], row), row, base64.b64decode(encoded, validate=True))
    assert request('GET', saved['route'] + '/administration')['administration']['administrative_status'] == 'closed'
    assert request('GET', '/api/v1/audit/verify')['valid'] is True
    print('Content restore passed: exact V1/V2 bytes and immutable Owner incident responses preserved.')


if __name__ == '__main__':
    assert len(sys.argv) == 2 and sys.argv[1] in ['capture', 'restore']
    capture() if sys.argv[1] == 'capture' else restore()
