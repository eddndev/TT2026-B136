#!/usr/bin/env python3
"""Compare complete dashboard snapshots across disposable database restoration."""
from datetime import datetime, timedelta, timezone
import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
from uuid import UUID

WORK = Path(os.environ['TT_DASHBOARD_API_WORK_DIR'])
BASE = os.environ['TT_DASHBOARD_API_BASE_URL']
STATE = WORK / 'dashboard-api-state.json'
COUNTERS = {'active_cases', 'pending_contracts', 'deadlines_overdue',
            'deadlines_due_48h', 'deadlines_due_7d', 'deadlines_unresolved'}


def request(token, expected=200):
    assert token.isascii() and not any(character in token for character in '\r\n')
    started = datetime.now(timezone.utc)
    with TemporaryDirectory(prefix='dashboard-http-', dir=WORK) as directory:
        headers, body = Path(directory) / 'headers', Path(directory) / 'body'
        result = subprocess.run([
            'curl', '--silent', '--show-error', '--max-time', '60', '--config', '-',
            '--dump-header', str(headers), '--output', str(body),
            '--write-out', '%{http_code}', BASE + '/api/v1/dashboard',
        ], input='header = ' + json.dumps('Authorization: Bearer ' + token) + '\n',
            text=True, capture_output=True, check=True)
        assert result.stdout == str(expected), ('dashboard status', result.stdout, expected)
        assert any(line.lower().strip() == 'cache-control: no-store'
                   for line in headers.read_text().splitlines()), 'Dashboard must not be cached'
        assert body.stat().st_size <= 2 * 1024 * 1024, 'Dashboard exceeds fixture response budget'
        value = json.loads(body.read_text())
    return value, started, datetime.now(timezone.utc)


def snapshot(token, scope):
    value, started, completed = request(token)
    assert set(value) == COUNTERS | {'checked_at', 'scope', 'workload'}
    assert value['scope'] == scope
    checked = datetime.fromisoformat(value['checked_at'].replace('Z', '+00:00'))
    assert checked.utcoffset() == timedelta(0)
    assert started - timedelta(seconds=1) <= checked <= completed + timedelta(seconds=1)
    assert all(type(value[key]) is int and value[key] >= 0 for key in COUNTERS)
    assert value['deadlines_due_48h'] <= value['deadlines_due_7d']
    assert isinstance(value['workload'], list)
    identifiers = []
    for row in value['workload']:
        assert set(row) == {'user_id', 'email', 'active_cases'}
        identifiers.append(str(UUID(row['user_id'])))
        assert isinstance(row['email'], str) and '@' in row['email']
        assert type(row['active_cases']) is int and 0 <= row['active_cases'] <= value['active_cases']
    assert identifiers == sorted(set(identifiers))
    return value


def collect():
    tokens = json.loads((WORK / 'deadlines-api-state.json').read_text())['agenda']['tokens']
    owner = snapshot(os.environ['TT_DASHBOARD_API_TOKEN'], 'office')
    assert owner['active_cases'] > 0 and owner['workload'], 'Expected populated office fixture'
    revoked = snapshot(tokens['litigator'], 'assigned_cases')
    assert all(revoked[key] == 0 for key in COUNTERS) and revoked['workload'] == []
    for role in ['paralegal', 'client']:
        denied, _, _ = request(tokens[role], 403)
        assert set(denied) == {'error'} and denied['error']['code'] == 'permission_denied'
    return {'owner': owner, 'revoked_litigator': revoked}


def run(action):
    snapshots = collect()
    if action == 'capture':
        STATE.write_text(json.dumps(snapshots, sort_keys=True))
        STATE.chmod(0o600)
        print('Dashboard API passed: populated office, empty revoked assignment scope and denied roles.')
        return
    saved = json.loads(STATE.read_text())
    assert saved.keys() == snapshots.keys()
    for actor, current in snapshots.items():
        before = saved[actor]
        assert datetime.fromisoformat(current['checked_at'].replace('Z', '+00:00')) >= \
            datetime.fromisoformat(before['checked_at'].replace('Z', '+00:00'))
        # Existing synthetic deadline fixtures use January and February 2026.
        # Compare temporal counters too: a changed fixture or crossed boundary
        # must fail visibly instead of being hidden by timestamp normalization.
        assert {key: value for key, value in current.items() if key != 'checked_at'} == \
            {key: value for key, value in before.items() if key != 'checked_at'}, \
            'Restored dashboard differs; inspect fixture changes and temporal boundaries'
    print('Dashboard restore passed: exact counters and workload; fresh UTC readings and role denial retained.')


if __name__ == '__main__':
    assert len(sys.argv) == 2 and sys.argv[1] in ['capture', 'restore']
    run(sys.argv[1])
