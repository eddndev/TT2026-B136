#!/usr/bin/env python3
"""Keep corpus preferences and MFA separate from the other disposable API fixtures."""
import json
import os
from pathlib import Path
import secrets
import sys
from uuid import uuid4

from api_procedural_facts_support import BASE, REPO, WORK, request

MATERIAL = WORK / 'legal-corpus-owner.json'
STATE = WORK / 'legal-corpus-api-state.json'


def save(value, exclusive=False):
    flags = os.O_WRONLY | os.O_CREAT | (os.O_EXCL if exclusive else os.O_TRUNC)
    with os.fdopen(os.open(MATERIAL, flags, 0o600), 'w', encoding='ascii') as stream:
        os.fchmod(stream.fileno(), 0o600)
        json.dump(value, stream, sort_keys=True, ensure_ascii=True)


def enroll():
    if MATERIAL.exists() or STATE.exists():
        raise SystemExit('Corpus capture already has private material; use verify or restore')
    password = secrets.token_urlsafe(32)
    row = request('POST', '/api/v1/users', {
        'email': 'api-legal-corpus-' + uuid4().hex + '@example.com',
        'password': password, 'role': 'owner',
    }, expected=201)
    saved = {'user_id': row['user']['id'], 'email': row['user']['email'],
             'password': password, 'recovery_codes': row['recovery_codes'],
             'next_recovery': 0, 'token': None}
    save(saved, exclusive=True)
    return saved


def login(saved):
    index = saved['next_recovery']
    if not 0 <= index < len(saved['recovery_codes']):
        raise SystemExit('Corpus Owner has no unused recovery code')
    challenge = request('POST', '/api/v1/auth/login', {
        'email': saved['email'], 'password': saved['password'],
    })
    session = request('POST', '/api/v1/auth/mfa/recovery', {
        'challenge_token': challenge['challenge_token'],
        'code': saved['recovery_codes'][index],
    })
    saved.update(token=session['access_token'], next_recovery=index + 1)
    save(saved)
    return saved['token']


def main(mode):
    saved = enroll() if mode == 'capture' else json.loads(MATERIAL.read_text(encoding='ascii'))
    token = saved['token']
    if mode == 'restore':
        if token:
            request('GET', '/api/v1/auth/me', token=token,
                    expected=401, code='invalid_session')
        token = login(saved)
    elif not token:
        token = login(saved)
    actor = request('GET', '/api/v1/auth/me', token=token)
    if actor['id'] != saved['user_id'] or actor['role'] != 'owner':
        raise SystemExit('Corpus session does not match its dedicated Owner')
    env = os.environ.copy()
    for prefix in ['TT_FACT_API', 'TT_DEADLINE_API']:
        env.update({prefix + '_BASE_URL': BASE, prefix + '_TOKEN': token,
                    prefix + '_WORK_DIR': str(WORK), prefix + '_REPO': str(REPO)})
    script = Path(REPO) / 'scripts/api-legal-corpus-demo.py'
    os.execve(sys.executable, [sys.executable, '-B', str(script), mode], env)


if __name__ == '__main__':
    if len(sys.argv) != 2 or sys.argv[1] not in ['capture', 'verify', 'restore']:
        raise SystemExit('Usage: api-legal-corpus-owner.py capture|verify|restore')
    main(sys.argv[1])
