"""Invalidate only the disposable identity sessions before database restoration."""
import base64
import hashlib
import hmac
import json
import math
import os
from pathlib import Path
import re
import struct
import subprocess
import sys
import time
from urllib.error import HTTPError
from urllib.request import Request, urlopen

WORK = Path(os.environ['TT_RESTORE_WORK']).resolve()
STATE = WORK / 'restored-identity.json'


def redis(*arguments):
    result = subprocess.run(['redis-cli', '-h', '127.0.0.1', '-p', os.environ['TT_RESTORE_REDIS_PORT'],
                             '--raw', *arguments], capture_output=True, text=True, timeout=10, check=True)
    return result.stdout.strip()


def invalidate():
    assert not os.environ.get('TT_RESTORE_SERVER_PID'), 'Stop the capture server first'
    process = dict(line.split(':', 1) for line in redis('INFO', 'server').splitlines() if ':' in line)
    assert process['process_id'] == os.environ['TT_RESTORE_REDIS_PID']
    directory = redis('CONFIG', 'GET', 'dir').splitlines()
    assert directory[0] == 'dir' and Path(directory[1]).resolve() == WORK
    for namespace in ['session', 'challenge', 'certificate-login']:
        cursor, keys = '0', set()
        for _ in range(1000):
            page = redis('SCAN', cursor, 'MATCH', f'identity:{namespace}:*', 'COUNT', '100').splitlines()
            cursor = page[0]
            for key in page[1:]:
                assert re.fullmatch('identity:' + namespace + r':[a-f0-9]{64}', key)
                keys.add(key)
            assert len(keys) <= 10000
            if cursor == '0':
                break
        else:
            raise AssertionError('Disposable authentication key scan did not finish')
        ordered = sorted(keys)
        for start in range(0, len(ordered), 100):
            redis('DEL', *ordered[start:start + 100])
    # Password and certificate budgets and TOTP replay claims are retained.
    print('Disposable restore: sessions, MFA and certificate captures invalidated; controls retained.')


def request(method, path, body=None, token=None, expected=200):
    headers = {} if token is None else {'Authorization': 'Bearer ' + token}
    if body is not None:
        headers['Content-Type'] = 'application/json'
        body = json.dumps(body).encode('ascii')
    try:
        response = urlopen(Request(os.environ['TT_RESTORE_BASE'] + '/api/v1' + path,
                                   data=body, headers=headers, method=method), timeout=60)
    except HTTPError as error:
        response = error
    with response:
        raw = response.read(32769)
        assert len(raw) <= 32768 and response.status == expected, (method, path, response.status, expected)
        return json.loads(raw) if raw else None


def fresh(email, password, secret):
    challenge = request('POST', '/auth/login', {'email': email, 'password': password})
    digest = hmac.new(base64.b32decode(secret), struct.pack('>Q', int(time.time()) // 30), hashlib.sha1).digest()
    offset = digest[-1] & 15
    value = (struct.unpack('>I', digest[offset:offset + 4])[0] & 0x7fffffff) % 1000000
    session = request('POST', '/auth/mfa/totp', {
        'challenge_token': challenge['challenge_token'], 'code': f'{value:06d}',
    })
    assert session['user']['email'] == email
    return session['access_token']


def wait_for_fresh_totp_window():
    # Restored accounts retain earlier replay claims. Start the whole login
    # batch in a new interval, before creating any one-use MFA challenge.
    started = time.monotonic()
    previous = time.time()
    assert math.isfinite(started) and math.isfinite(previous) and previous >= 0, 'Invalid authentication clock'
    boundary = (int(previous) // 30 + 1) * 30
    delay = boundary - previous
    assert 0 < delay <= 30, 'Invalid TOTP boundary wait'
    time.sleep(delay)
    elapsed = time.monotonic() - started
    current = time.time()
    assert math.isfinite(elapsed) and 0 <= elapsed <= 31, 'TOTP boundary wait exceeded its clock budget'
    assert math.isfinite(current) and boundary <= current < boundary + 30, 'TOTP clock did not enter the expected interval'


def login():
    for name in ['TT_RESTORE_OLD_OWNER', 'TT_RESTORE_OLD_HELPER']:
        request('GET', '/auth/me', token=os.environ[name], expected=401)
    wait_for_fresh_totp_window()
    result = {
        'owner': fresh('owner@example.com', 'correct horse battery staple', os.environ['TT_RESTORE_OWNER_SECRET']),
        'helper': fresh('helper@example.com', 'another safe password', os.environ['TT_RESTORE_HELPER_SECRET']),
    }
    fixture = WORK / 'deadline-login-material.json'
    if fixture.exists():
        credentials = json.loads(fixture.read_text())
        path = WORK / 'deadlines-api-state.json'
        saved = json.loads(path.read_text())
        for role, account in credentials.items():
            request('GET', '/auth/me', token=saved['agenda']['tokens'][role], expected=401)
            saved['agenda']['tokens'][role] = fresh(account['email'], account['password'], account['secret'])
        path.write_text(json.dumps(saved, sort_keys=True))
        path.chmod(0o600)
    STATE.write_text(json.dumps(result))
    STATE.chmod(0o600)
    print('Disposable restore: fresh MFA sessions replace revoked values without changing account revisions.')


if __name__ == '__main__':
    assert len(sys.argv) == 2 and sys.argv[1] in ['invalidate', 'login']
    invalidate() if sys.argv[1] == 'invalidate' else login()
