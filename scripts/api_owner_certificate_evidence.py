"""Public evidence checks for the disposable Owner binding HTTP campaign."""
import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen
from uuid import UUID

WORK = Path(os.environ['TT_OWNER_CERT_WORK'])
STATE = WORK / 'owner-certificate-api-state.json'
TOKEN = os.environ['TT_OWNER_CERT_TOKEN']
PREFIX = '/api/v1/auth/certificate-bindings/'
LIMIT = 2 * 1024 * 1024


def command(arguments):
    result = subprocess.run(arguments, capture_output=True, timeout=30, check=False)
    assert result.returncode == 0, 'Owner fixture subprocess failed'
    return result.stdout


def request(method, path, body=None, expected=200, code=None, token=TOKEN):
    headers = {'Authorization': 'Bearer ' + token}
    if body is not None:
        body = json.dumps(body, ensure_ascii=True, separators=(',', ':')).encode('ascii')
        headers['Content-Type'] = 'application/json'
    query = Request(os.environ['TT_OWNER_CERT_BASE'] + path, data=body, headers=headers, method=method)
    try:
        response = urlopen(query, timeout=60)
    except HTTPError as error:
        response = error
    except URLError:
        raise AssertionError('Owner fixture HTTP transport failed') from None
    with response:
        raw = response.read(LIMIT + 1)
        assert len(raw) <= LIMIT, 'Owner fixture response exceeds its public evidence budget'
        value = json.loads(raw) if raw else None
        actual = (value or {}).get('error', {}).get('code')
        assert response.status == expected, (method, path, response.status, expected, actual)
        assert response.headers.get('Cache-Control') == 'no-store'
        assert not any(response.headers.get(name) for name in ['Authorization', 'Set-Cookie', 'Location'])
        if code:
            assert actual == code, (path, actual, code)
    return value


def encoded(value):
    return base64.b64encode(value).decode('ascii')


def decoded(value):
    result = base64.b64decode(value, validate=True)
    assert encoded(result) == value, 'Noncanonical public base64'
    return result


def canonical(binding, owner, revision, generation, deployment, root, trust, leaf, withdrawal=False):
    result = (b'OWNCERT1' + bytes([2 if withdrawal else 1, 1]) + UUID(deployment).bytes
              + bytes.fromhex(root) + int(trust).to_bytes(4, 'big') + UUID(owner).bytes
              + int(revision).to_bytes(8, 'big') + int(generation).to_bytes(8, 'big')
              + UUID(binding).bytes + (1 if withdrawal else 0).to_bytes(4, 'big')
              + (2 if withdrawal else 1).to_bytes(4, 'big') + bytes.fromhex(leaf))
    assert len(result) == 150
    return result


def prepared_bytes(value):
    result = canonical(value['binding_id'], value['owner_id'], value['account_revision'],
                       value['auth_generation'], value['deployment_id'], value['root_fingerprint'],
                       value['trust_revision'], value['certificate']['fingerprint'])
    assert decoded(value['statement_base64']) == result
    assert value['policy'] == 'internal_partner_binding_v1'
    assert hashlib.sha256(decoded(value['certificate']['der_base64'])).hexdigest() == value['certificate']['fingerprint']
    return result


def sign(prepared):
    statement = prepared_bytes(prepared)
    path = WORK / 'owner-client' / 'statement.bin'
    path.write_bytes(statement)
    signature = command(['openssl', 'dgst', '-sha256', '-sign',
                         os.environ['TT_OWNER_CERT_KEY'], str(path)])
    assert len(signature) == 384
    return {'statement_base64': encoded(statement),
            'certificate_der_base64': prepared['certificate']['der_base64'],
            'signature_base64': encoded(signature)}


def verify_receipt(value, intent):
    row, certificate = value['registration'], value['registration']['certificate']
    trust = row['trust']
    assert value['policy'] == 'internal_partner_binding_v1'
    assert row['statement_base64'] == intent['statement_base64']
    assert certificate['der_base64'] == intent['certificate_der_base64']
    assert row['signature_base64'] == intent['signature_base64']
    statement, der = decoded(row['statement_base64']), decoded(certificate['der_base64'])
    assert statement == canonical(value['binding_id'], value['owner_id'], row['account_revision'],
                                  row['auth_generation'], trust['deployment_id'], trust['root_fingerprint'],
                                  trust['revision'], certificate['fingerprint'])
    assert hashlib.sha256(statement).hexdigest() == row['statement_digest']
    assert hashlib.sha256(der).hexdigest() == certificate['fingerprint']
    assert hashlib.sha256(decoded(trust['root_der_base64'])).hexdigest() == trust['root_fingerprint']
    assert hashlib.sha256(decoded(trust['crl_der_base64'])).hexdigest() == trust['crl_digest']
    assert isinstance(trust['crl_number'], str)
    for name in ['account_revision', 'auth_generation']:
        assert isinstance(row[name], str) and str(int(row[name])) == row[name]
    assert row['valid_from_unix'] <= row['checked_at_unix'] <= row['valid_until_unix']
    directory = WORK / 'owner-client'
    (directory / 'public.der').write_bytes(der)
    (directory / 'public.bin').write_bytes(statement)
    (directory / 'public.sig').write_bytes(decoded(row['signature_base64']))
    public = command(['openssl', 'x509', '-inform', 'DER', '-in', str(directory / 'public.der'), '-pubkey', '-noout'])
    (directory / 'public.pem').write_bytes(public)
    command(['openssl', 'dgst', '-sha256', '-verify', str(directory / 'public.pem'),
             '-signature', str(directory / 'public.sig'), str(directory / 'public.bin')])
    withdrawal = value['withdrawal']
    assert value['revision'] == (2 if withdrawal else 1)
    if withdrawal:
        assert decoded(withdrawal['statement_base64']) == canonical(
            value['binding_id'], value['owner_id'], withdrawal['account_revision'], withdrawal['auth_generation'],
            trust['deployment_id'], trust['root_fingerprint'], trust['revision'], certificate['fingerprint'], True)


def sql_state(owner):
    identifier = str(UUID(owner))
    query = """SELECT jsonb_build_object(
      'registrations',COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY binding_id)
        FROM owner_certificate_registrations r),'[]'::jsonb),
      'withdrawals',COALESCE((SELECT jsonb_agg(to_jsonb(w) ORDER BY binding_id)
        FROM owner_certificate_withdrawals w),'[]'::jsonb),
      'events',COALESCE((SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a
        WHERE action IN ('identity.owner_certificate_registered','identity.owner_certificate_withdrawn')),'[]'::jsonb),
      'account',(SELECT jsonb_build_object('id',id,'email',email,'role',role,'active',active,
        'revision',revision::text,'auth_generation',auth_generation::text) FROM users WHERE id='%s'))""" % identifier
    raw = command(['psql', os.environ['TT_OWNER_CERT_DATABASE'], '-X', '-qAt', '-v', 'ON_ERROR_STOP=1', '-c', query])
    return json.loads(raw)


def verify_sql(value, intent, owner):
    state = sql_state(owner['id'])
    assert len(state['registrations']) == 1
    assert len(state['withdrawals']) == (1 if value['withdrawal'] else 0)
    assert len(state['events']) == value['revision']
    registration = state['registrations'][0]
    assert registration['binding_id'] == value['binding_id'] and registration['owner_id'] == owner['id']
    for sql_name, input_name in [('statement', 'statement_base64'), ('certificate_der', 'certificate_der_base64'),
                                 ('signature', 'signature_base64')]:
        assert registration[sql_name] == '\\x' + decoded(intent[input_name]).hex()
    for index, event in enumerate(state['events']):
        withdrawn = index == 1
        row = value['withdrawal'] if withdrawn else value['registration']
        raw = decoded(row['statement_base64'])
        action = 'identity.owner_certificate_' + ('withdrawn' if withdrawn else 'registered')
        assert event['action'] == action and event['actor'] == owner['email']
        assert event['resource'] == ('owner-certificate:' + value['binding_id'] + ':owner:' + owner['id']
                                     + ':revision:' + str(index + 1) + ':statement:' + hashlib.sha256(raw).hexdigest())
        assert event['timestamp'] == row['withdrawn_at' if withdrawn else 'registered_at']
        stored = state['withdrawals'][0] if withdrawn else registration
        assert stored['statement'] == '\\x' + raw.hex()
        assert stored['statement_digest'] == '\\x' + hashlib.sha256(raw).hexdigest()
        assert stored['audit_sequence'] == event['sequence']
    assert request('GET', '/api/v1/audit/verify')['valid'] is True
    return state


def unchanged(owner, before, method, path, body=None, **options):
    result = request(method, path, body, **options)
    assert sql_state(owner) == before, 'Rejected or reconciled Owner request changed durable state'
    return result


def save(value):
    STATE.write_text(json.dumps(value, sort_keys=True, ensure_ascii=True) + '\n')
    STATE.chmod(0o600)
