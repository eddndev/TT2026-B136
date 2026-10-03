"""Populate real identity state and reset SQL contracts without external delivery."""
import base64
import hashlib
import hmac
import os
import json
import secrets
import struct
import time
import uuid
from datetime import timezone
from contextlib import closing
from pathlib import Path

import psycopg2


def connection(fixture):
    return psycopg2.connect(host='127.0.0.1', port=fixture.config['postgres_port'],
                            dbname='qadra', user='qadra_admin',
                            password=fixture.config['admin_password'], connect_timeout=5)


def query(fixture, statement, args=()):
    with closing(connection(fixture)) as db, db:
        with db.cursor() as cursor:
            cursor.execute(statement, args)
            return cursor.fetchall() if cursor.description else []


def digest_token():
    token = secrets.token_bytes(32)
    return token, hashlib.sha256(b'qadra:password-reset:v1\0' + token).digest()


def issue(fixture, email, ttl=600):
    token, digest = digest_token()
    identifier = str(uuid.uuid4())
    rows = query(fixture, 'SELECT * FROM public.password_reset_issue(%s,%s,%s,%s,%s)',
                 (identifier, email, digest, ttl, 8))
    fixture.assertEqual(len(rows), 1)
    return {'id': identifier, 'token': token, 'digest': digest}


def consume(fixture, capability, user, password_hash):
    with closing(connection(fixture)) as db, db:
        with db.cursor() as cursor:
            cursor.execute('SELECT pg_catalog.pg_advisory_xact_lock(%s)', (0x4155444954,))
            cursor.execute('SELECT auth_generation FROM public.users WHERE id=%s', (user,))
            generation, = cursor.fetchone()
            cursor.execute('SELECT * FROM public.password_reset_consume(%s,%s,%s,%s,%s)',
                           (capability['id'], capability['digest'], user, generation, password_hash))
            at, sequence, revision, generation = cursor.fetchone()
            cursor.execute('SELECT chain FROM public.audit_events ORDER BY sequence DESC LIMIT 1')
            previous = cursor.fetchone()
            previous = bytes(previous[0]) if previous else bytes(32)
            at = at.astimezone(timezone.utc)
            timestamp = at.strftime('%Y-%m-%dT%H:%M:%S')
            if at.microsecond:
                timestamp += '.' + f'{at.microsecond:06d}'.rstrip('0')
            timestamp += 'Z'
            actor, action = 'password-reset', 'identity.password_reset'
            resource = f'user:{user}:revision:{revision}:generation:{generation}'
            data = struct.pack('>Q', sequence)
            for field in (timestamp, actor, action, resource):
                encoded = field.encode()
                data += struct.pack('>I', len(encoded)) + encoded
            chain = hashlib.sha256(previous + data).digest()
            cursor.execute('INSERT INTO public.audit_events(sequence,timestamp,actor,action,resource,chain) VALUES(%s,%s,%s,%s,%s,%s)',
                           (sequence, timestamp, actor, action, resource, chain))
    return revision, generation


def login(fixture, email, password):
    return fixture.request('/auth/login', {'email': email, 'password': password})['challenge_token']


def recover(fixture, challenge, code, status=200):
    value = fixture.request('/auth/mfa/recovery', {'challenge_token': challenge, 'code': code}, status=status)
    return value.get('access_token')


def totp(secret):
    key = base64.b32decode(secret + '=' * (-len(secret) % 8))
    raw = hmac.new(key, struct.pack('>Q', int(time.time()) // 30), hashlib.sha1).digest()
    offset = raw[-1] & 15
    return f'{(struct.unpack(">I", raw[offset:offset+4])[0] & 0x7fffffff) % 1000000:06d}'


def seed(fixture):
    email = 'restore-owner@example.test'
    old_password, password = secrets.token_hex(20), secrets.token_hex(20)
    enrolled = fixture.request('/auth/bootstrap', {'email': email, 'password': old_password}, status=201)
    owner_session = recover(fixture, login(fixture, email, old_password), enrolled['recovery_codes'][0])
    other = fixture.request('/users', {'email': 'restore-helper@example.test',
                                      'password': password, 'role': 'litigator'}, owner_session, status=201)
    case = fixture.request('/cases', {'title': 'Private restoration fixture', 'reference': 'RESTORE-OWNED'}, owner_session, status=201)
    helper, = query(fixture, 'SELECT id::text FROM users WHERE email=%s', ('restore-helper@example.test',))[0]
    fixture.request('/cases/' + case['id'] + '/members/' + helper, token=owner_session, status=204, method='PUT')
    user, = query(fixture, 'SELECT id::text FROM users WHERE email=%s', (email,))[0]
    replacement_hash, = query(fixture, 'SELECT password_hash FROM users WHERE email=%s',
                               ('restore-helper@example.test',))[0]
    consumed = issue(fixture, email)
    consume(fixture, consumed, user, replacement_hash)
    expired = issue(fixture, email, ttl=1)
    cancelled = issue(fixture, email)
    query(fixture, 'SELECT public.password_reset_cancel(%s,%s)', (cancelled['id'], cancelled['digest']))
    pending = issue(fixture, email)
    # Issue active credentials only after the consumed reset advances generation.
    challenge = login(fixture, email, password)
    code = totp(enrolled['totp_secret_base32'])
    session = fixture.request('/auth/mfa/totp', {'challenge_token': challenge, 'code': code})['access_token']
    waiting = login(fixture, email, password)
    fixture.request('/auth/login', {'email': email, 'password': old_password}, status=401)
    email_digest = hashlib.sha256(b'qadra:password-reset-request-limit:v1\0' + email.encode()).hexdigest()
    token_digest = hashlib.sha256(b'qadra:password-reset-completion-limit:v1\0' + pending['digest']).hexdigest()
    quotas = ['identity:password-reset:v1:request:global',
              'identity:password-reset:v1:request:email:' + email_digest,
              'identity:password-reset:v1:completion:global',
              'identity:password-reset:v1:completion:digest:' + token_digest]
    budget = (Path(__file__).resolve().parents[2] /
              'crates/infrastructure/src/identity/password_reset_runtime/budget.lua').read_text()
    for offset in (0, 2):
        fixture.assertEqual(fixture.cli('EVAL', budget, 2, *quotas[offset:offset+2],
                                       8, 600000, 8, 600000), 1)
    fixture.assertEqual(fixture.cli('SET', 'unrelated:restore-preserved', '1', 'PX', 600000), 'OK')
    expiry_wait = time.monotonic() + 5
    while query(fixture, 'SELECT expires_at<=clock_timestamp() FROM password_reset_capabilities WHERE id=%s', (expired['id'],))[0][0] is False:
        fixture.assertLess(time.monotonic(), expiry_wait, 'owned reset did not expire')
        time.sleep(0.05)
    return {'user': user, 'email': email, 'password': password, 'codes': enrolled['recovery_codes'],
            'totp': enrolled['totp_secret_base32'], 'session': session, 'challenge': waiting,
            'quotas': quotas, 'budget_script': budget,
            'capabilities': {'consumed': consumed, 'expired': expired, 'cancelled': cancelled, 'pending': pending}}


def identity_snapshot(fixture):
    return query(fixture, "SELECT jsonb_build_object('users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),'members',(SELECT jsonb_agg(to_jsonb(m) ORDER BY case_id,user_id) FROM case_memberships m),'capabilities',(SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM password_reset_capabilities c),'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))")[0][0]


def redis_snapshot(fixture):
    cursor, result = '0', {}
    for _ in range(100):
        cursor, keys = fixture.cli('SCAN', cursor, 'COUNT', 100)
        for key in keys:
            kind = fixture.cli('TYPE', key)
            fixture.assertIn(kind, ('string', 'hash'), 'unexpected owned Redis value type')
            args = [fixture.tools['redis_cli'], '-h', '127.0.0.1', '-p', str(fixture.port), '-2']
            args.extend(['--raw', 'GET', key] if kind == 'string' else ['--json', 'HGETALL', key])
            raw = fixture.command(args, env={**os.environ, 'REDISCLI_AUTH': fixture.password}, timeout=10).stdout
            fixture.assertTrue(raw.endswith(b'\n') and len(raw) <= 65536, 'owned Redis value exceeds its boundary')
            if kind == 'hash':
                fields = json.loads(raw)
                fixture.assertTrue(len(fields) % 2 == 0 and all(isinstance(v, str) for v in fields))
                pairs = list(zip(fields[::2], fields[1::2]))
                fixture.assertEqual(len(pairs), len(dict(pairs)))
                value = json.dumps(sorted(pairs), ensure_ascii=True, separators=(',', ':')).encode()
            else:
                value = raw[:-1]
            result[key] = (kind.encode() + b'\0' + value, fixture.cli('PEXPIRETIME', key))
        if str(cursor) == '0':
            return result
    fixture.fail('owned native Redis inventory did not finish')
