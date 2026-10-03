#!/usr/bin/env python3
"""Exercise real Owner binding HTTP, public proof, revocation and restored history."""
import copy
import json
import os
from pathlib import Path
import sys
from uuid import uuid4

from api_owner_certificate_evidence import (
    PREFIX, STATE, WORK, command, decoded, encoded, prepared_bytes, request, save, sign,
    sql_state, unchanged, verify_receipt, verify_sql,
)


def preparation(binding, owner):
    path = Path(os.environ['TT_OWNER_CERT_CERTIFICATE'])
    certificate = path.read_bytes()
    value = request('POST', PREFIX + binding + '/prepare', {'certificate_base64': encoded(certificate)})
    assert value['binding_id'] == binding and value['owner_id'] == owner['id']
    assert decoded(value['certificate']['der_base64']) == command(['openssl', 'x509', '-in', str(path), '-outform', 'DER'])
    prepared_bytes(value)
    return value


def capture():
    owner = request('GET', '/api/v1/auth/me')
    assert owner['role'] == 'owner'
    before = sql_state(owner['id'])
    assert before['registrations'] == before['withdrawals'] == before['events'] == []
    binding, next_binding = str(uuid4()), str(uuid4())
    prepared = preparation(binding, owner)
    assert prepared['account_revision'] == before['account']['revision']
    assert prepared['auth_generation'] == before['account']['auth_generation']
    intent = sign(prepared)
    assert sql_state(owner['id']) == before, 'Preparation wrote Owner state'
    invalid = copy.deepcopy(intent)
    signature = decoded(invalid['signature_base64'])
    invalid['signature_base64'] = encoded(bytes([signature[0] ^ 1]) + signature[1:])
    route = PREFIX + binding
    unchanged(owner['id'], before, 'POST', route + '/register', invalid,
              expected=422, code='owner_certificate_credential_rejected')
    receipt = request('POST', route + '/register', intent)
    assert receipt['owner_id'] == owner['id'] and receipt['binding_id'] == binding
    assert receipt['revision'] == 1 and receipt['withdrawal'] is None
    verify_receipt(receipt, intent)
    assert receipt['registration']['trust']['revision'] == prepared['trust_revision']
    committed = verify_sql(receipt, intent, owner)
    assert committed['account'] == before['account'], 'Registration changed account state'
    assert unchanged(owner['id'], committed, 'GET', route) == receipt
    assert unchanged(owner['id'], committed, 'POST', route + '/register', intent) == receipt
    conflict = copy.deepcopy(intent)
    statement = decoded(conflict['statement_base64'])
    conflict['statement_base64'] = encoded(bytes([statement[0] ^ 1]) + statement[1:])
    unchanged(owner['id'], committed, 'POST', route + '/register', conflict,
              expected=409, code='owner_certificate_conflict')
    next_prepared = preparation(next_binding, owner)
    next_intent = sign(next_prepared)
    assert sql_state(owner['id']) == committed
    assert next_prepared['trust_revision'] == prepared['trust_revision']
    save({'owner': owner, 'prepared': prepared, 'intent': intent, 'receipt': receipt,
          'next_binding': next_binding, 'next_intent': next_intent, 'state': committed})
    print('Owner API registration passed: exact public intent, RSA proof, rejected signature and one audited commit.')


def denied_authority(saved, state):
    route = PREFIX + saved['receipt']['binding_id']
    certificate = encoded(Path(os.environ['TT_OWNER_CERT_CERTIFICATE']).read_bytes())
    operations = [('POST', route + '/prepare', {'certificate_base64': certificate}),
                  ('POST', route + '/register', saved['intent']), ('GET', route, None),
                  ('POST', route + '/withdraw', {'expected_revision': 1})]
    for token, status, code in [(os.environ['TT_OWNER_CERT_REVOKED'], 401, 'invalid_session'),
                                (os.environ['TT_OWNER_CERT_PARALEGAL'], 403, 'permission_denied')]:
        for method, path, body in operations:
            unchanged(saved['owner']['id'], state, method, path, body,
                      token=token, expected=status, code=code)


def rotated():
    saved = json.loads(STATE.read_text())
    owner, registered, intent = saved['owner'], saved['receipt'], saved['intent']
    assert request('GET', '/api/v1/auth/me') == owner
    publication = json.loads((WORK / 'owner-certificate-trust.json').read_text())
    old_trust = registered['registration']['trust']
    assert publication['revision'] == old_trust['revision'] + 1
    assert publication['deployment_id'] == old_trust['deployment_id']
    assert publication['root_fingerprint'] == old_trust['root_fingerprint']
    assert publication['crl_digest'] != old_trust['crl_digest']
    assert int(publication['crl_number']) > int(old_trust['crl_number'])
    assert sql_state(owner['id']) == saved['state'], 'Trust publication changed Owner bindings'
    route, next_route = PREFIX + registered['binding_id'], PREFIX + saved['next_binding']
    unchanged(owner['id'], saved['state'], 'POST', next_route + '/register', saved['next_intent'],
              expected=409, code='owner_certificate_conflict')
    assert unchanged(owner['id'], saved['state'], 'GET', route) == registered
    assert unchanged(owner['id'], saved['state'], 'POST', route + '/register', intent) == registered
    terminal = request('POST', route + '/withdraw', {'expected_revision': 1})
    assert terminal['registration'] == registered['registration']
    assert terminal['revision'] == 2 and terminal['withdrawal'] is not None
    verify_receipt(terminal, intent)
    state = verify_sql(terminal, intent, owner)
    assert state['account'] == saved['state']['account'], 'Withdrawal changed account state'
    assert unchanged(owner['id'], state, 'POST', route + '/withdraw', {'expected_revision': 1}) == terminal
    assert unchanged(owner['id'], state, 'POST', route + '/register', intent) == terminal
    current = preparation(saved['next_binding'], owner)
    assert current['trust_revision'] == publication['revision']
    revoked_intent = sign(current)
    assert sql_state(owner['id']) == state
    unchanged(owner['id'], state, 'POST', next_route + '/register', revoked_intent,
              expected=422, code='owner_certificate_credential_rejected')
    unchanged(owner['id'], state, 'GET', next_route, expected=404, code='owner_certificate_not_found')
    denied_authority(saved, state)
    saved.update(receipt=terminal, state=state, current_intent=revoked_intent)
    save(saved)
    print('Owner API revocation passed: stale trust conflicts, new revoked proof is rejected and withdrawal preserves history.')


def restore():
    saved = json.loads(STATE.read_text())
    owner, receipt, intent = saved['owner'], saved['receipt'], saved['intent']
    assert request('GET', '/api/v1/auth/me') == owner
    before = sql_state(owner['id'])
    assert before == saved['state'], 'Restored Owner tables, events or account capture differ'
    route = PREFIX + receipt['binding_id']
    assert unchanged(owner['id'], before, 'GET', route) == receipt
    assert unchanged(owner['id'], before, 'POST', route + '/register', intent) == receipt
    assert unchanged(owner['id'], before, 'POST', route + '/withdraw', {'expected_revision': 1}) == receipt
    unchanged(owner['id'], before, 'GET', PREFIX + saved['next_binding'],
              expected=404, code='owner_certificate_not_found')
    unchanged(owner['id'], before, 'POST', PREFIX + saved['next_binding'] + '/register',
              saved['current_intent'], expected=422, code='owner_certificate_credential_rejected')
    verify_receipt(receipt, intent)
    assert verify_sql(receipt, intent, owner) == before
    denied_authority(saved, before)
    print('Owner restore passed: fresh MFA, exact terminal receipt, original public proof and unchanged audit evidence.')


if __name__ == '__main__':
    assert len(sys.argv) == 2 and sys.argv[1] in ['capture', 'rotated', 'restore']
    {'capture': capture, 'rotated': rotated, 'restore': restore}[sys.argv[1]]()
