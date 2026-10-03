"""Run one explicit populated restoration using an independently prepared test release."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import time
import unittest
import uuid
from unittest.mock import patch

from native_restore_backends import NativeRestore
from native_restore_identity import seed, query, identity_snapshot, redis_snapshot, login, recover
import restore_admission
import restore_capture
import restore_catalog
import restore_commands
import restore_fence
import restore_observations
import restore_redis
import runtime


class PopulatedRestore(NativeRestore):
    result = None

    def capture_command(self, args, **kwargs):
        if str(args[0]) == 'systemctl':
            return self.service_observation(args, **kwargs)
        return restore_commands.run(args, **kwargs)

    def cli_reset(self, descriptor, operation):
        head = descriptor['audit_predecessor']
        result = self.command([self.target / 'bin/despacho-cli', '--json', 'database',
            'invalidate-restored-password-resets', '--operation-id', operation,
            '--expected-database', 'qadra', '--expected-schema', 'public',
            '--expected-audit-sequence', str(head['sequence']), '--expected-audit-head', head['head']],
            env=runtime.environment(self.root, admin=True), cwd=self.root / 'data')
        return json.loads(result.stdout)

    def test_populated_capture_restore_and_authentication(self):
        begin = time.monotonic()
        credentials = seed(self)
        before = identity_snapshot(self)
        self.assertEqual(len(before['members']), 2)
        self.assertEqual(len(before['capabilities']), 4)
        self.assertEqual(sum(row['consumed_at'] is not None for row in before['capabilities']), 1)
        self.assertEqual(sum(row['cancelled_at'] is not None for row in before['capabilities']), 1)
        redis_before = redis_snapshot(self)
        self.stop_api()
        descriptor_path = self.root / 'receipts/compatibility.json'
        with patch.object(restore_capture, 'run', side_effect=self.capture_command), \
                patch.object(restore_observations, 'run', side_effect=self.capture_command):
            captured = restore_capture.capture(self.root, controller_directory=Path(runtime.__file__).parent,
                                                targets=self.targets(), tools=self.tools, descriptor_path=descriptor_path)
        descriptor = json.loads(descriptor_path.read_text())
        self.assertEqual(descriptor['postgres']['server_version_num'] // 10000, 16)
        self.assertEqual(descriptor['redis']['engine'], 'redis')
        self.assertEqual(descriptor['redis']['server_version'], '7.4.11')
        backup = captured['backup']
        operation = str(uuid.uuid4())
        restore_fence.enter(self.root, operation)
        with patch.object(restore_observations, 'run', side_effect=self.capture_command):
            observed, _ = restore_observations.observe(self.root, self.targets(), self.tools)
        with (backup / 'redis.rdb').open('rb') as stream:
            observed['redis']['rdb_version'] = int(stream.read(9)[5:])
        accepted = restore_admission.admit(self.root, backup, descriptor_path,
            expected_descriptor_sha256=captured['descriptor_sha256'],
            controller_directory=Path(runtime.__file__).parent, observed=observed)
        self.assertTrue(accepted['audit_predecessor'] == descriptor['audit_predecessor'])
        self.check_private_archive(backup)
        env = {**runtime.environment(self.root, admin=True), 'PGDATABASE': 'postgres'}
        self.command([self.tools['psql'], '-X', '-v', 'ON_ERROR_STOP=1'], env=env,
                     input=b'DROP DATABASE qadra; CREATE DATABASE qadra OWNER qadra_admin;')
        self.command([self.tools['pg_restore'], '--single-transaction', '--exit-on-error',
                      '--dbname=qadra', backup / 'database.dump'], env=runtime.environment(self.root, admin=True))
        self.command([self.target / 'bin/despacho-cli', 'database', 'check'],
                     env=runtime.environment(self.root), cwd=self.root / 'data')
        self.assertTrue(identity_snapshot(self) == before, 'SQL restoration changed captured identity or audit bytes')
        forged = {**descriptor, 'audit_predecessor': {**descriptor['audit_predecessor'], 'head': '0' * 64}}
        with self.assertRaisesRegex(AssertionError, 'owned native command failed'):
            self.cli_reset(forged, operation)
        self.assertTrue(identity_snapshot(self) == before, 'rejected predecessor changed SQL state')
        first = self.cli_reset(descriptor, operation)
        second = self.cli_reset(descriptor, operation)
        self.assertTrue(first['applied'] and not second['applied'])
        self.assertEqual(first['invalidated'], 2)
        self.assertTrue({k:v for k,v in first.items() if k != 'applied'} ==
                        {k:v for k,v in second.items() if k != 'applied'}, 'uncertain retry returned another receipt')
        after = identity_snapshot(self)
        self.assertTrue(after['users'] == before['users'] and after['members'] == before['members'],
                        'invalidation changed identity or memberships')
        self.assertTrue(after['audit'][:-1] == before['audit'], 'invalidation changed the audit prefix')
        self.assertEqual(len(after['audit']), len(before['audit']) + 1)
        old = {row['id']: row for row in before['capabilities']}
        self.assertEqual(set(old), {row['id'] for row in after['capabilities']})
        self.assertEqual(len(old), len(after['capabilities']))
        for row in after['capabilities']:
            previous = old[row['id']]
            if previous['consumed_at'] or previous['cancelled_at']:
                self.assertTrue(row == previous, 'terminal capability changed during invalidation')
            else:
                self.assertIsNotNone(row['cancelled_at'])
                self.assertTrue({k:v for k,v in row.items() if k != 'cancelled_at'} ==
                                {k:v for k,v in previous.items() if k != 'cancelled_at'},
                                'invalidation changed more than cancellation time')
        verified, _ = restore_capture.verified_audit(self.root, self.tools, self.targets()['postgres'], self.target)
        self.assertEqual(len(verified), len(after['audit']))
        self.restore_redis(backup, redis_before)
        for offset in (0, 2):
            keys = credentials['quotas'][offset:offset+2]
            deadlines = [self.cli('PEXPIRETIME', key) for key in keys]
            self.assertEqual(self.cli('EVAL', credentials['budget_script'], 2, *keys, 8, 600000, 8, 600000), 1)
            self.assertEqual([self.cli('HGET', key, 'count') for key in keys], ['2', '2'])
            self.assertEqual([self.cli('PEXPIRETIME', key) for key in keys], deadlines)
        for capability in credentials['capabilities'].values():
            self.assertEqual(query(self, 'SELECT count(*) FROM password_reset_inspect(%s)', (capability['digest'],))[0][0], 0)
        self.start_api()
        self.request('/auth/me', token=credentials['session'], status=401)
        recover(self, credentials['challenge'], credentials['codes'][1], status=401)
        new_challenge = login(self, credentials['email'], credentials['password'])
        new_session = recover(self, new_challenge, credentials['codes'][1])
        self.request('/auth/me', token=new_session)
        self.stop_api()
        self.assertTrue((self.root / 'maintenance/restore/active.json').is_file())
        self.result.update(status='assertions_passed', seconds=round(time.monotonic()-begin, 3),
                           postgres=descriptor['postgres']['server_version_num'], redis=descriptor['redis']['server_version'],
                           users=len(before['users']), reset_states=4, reset_invalidated=2,
                           original_audit_entries=len(before['audit']), audit_delta=1,
                           restored_controls=len(redis_before)-3, receipt_retry='same operation; applied=false',
                           public_ingress=False, external_delivery=False,
                           service_supervisor='owned-process observation double',
                           reset_seeding='native SQL contract plus exact cryptographic audit receipt')

    def check_private_archive(self, backup):
        stage = self.root / 'receipts/private-copy'
        stage.mkdir(mode=0o700)
        with tarfile.open(backup / 'private-state.tar.gz') as archive:
            members = archive.getmembers()
            self.assertLessEqual(len(members), 10000)
            self.assertLessEqual(sum(m.size for m in members), 32 * 1024 * 1024)
            for member in members:
                path = Path(member.name)
                self.assertTrue(not path.is_absolute() and '..' not in path.parts and
                                (member.isdir() or member.isfile()) and
                                any(path == Path(base) or Path(base) in path.parents for base in ('config', 'data/ca', 'data/tsa')),
                                'unexpected private archive member')
            archive.extractall(stage, filter='data')
        expected = {}
        for prefix in ('config', 'data/ca', 'data/tsa'):
            for path in (self.root / prefix).rglob('*'):
                if path.is_file():
                    expected[path.relative_to(self.root).as_posix()] = hashlib.sha256(path.read_bytes()).digest()
        actual = {p.relative_to(stage).as_posix(): hashlib.sha256(p.read_bytes()).digest()
                  for p in stage.rglob('*') if p.is_file()}
        self.assertTrue(actual == expected, 'private file inventory or content changed')
        preserved = self.root / 'receipts/private-before-restore'
        for prefix in ('config', 'data/ca', 'data/tsa'):
            original = self.root / prefix
            old = preserved / prefix
            old.parent.mkdir(parents=True, exist_ok=True)
            original.rename(old)
            shutil.copytree(stage / prefix, original)

    def restore_redis(self, backup, original):
        self.stop()
        shutil.copyfile(backup / 'redis.rdb', self.root / 'data/redis.rdb')
        self.start(False)
        observed = redis_snapshot(self)
        if observed != original:
            comparison = {'missing': sorted(set(original)-set(observed)), 'extra': sorted(set(observed)-set(original)),
                          'changed': [{'key': key, 'old_expiry': original[key][1], 'new_expiry': observed[key][1],
                                       'old_hash': hashlib.sha256(original[key][0]).hexdigest(),
                                       'new_hash': hashlib.sha256(observed[key][0]).hexdigest()}
                                      for key in original.keys() & observed.keys() if original[key] != observed[key]]}
            self.diagnostics.mkdir(mode=0o700, parents=True, exist_ok=True)
            (self.diagnostics / 'redis-comparison.json').write_text(json.dumps(comparison, indent=2))
        self.assertTrue(observed == original, 'RDB changed values or absolute expiry')
        removed = restore_redis.invalidate_sessions(redis_cli=self.tools['redis_cli'], host='127.0.0.1',
            port=self.port, password=self.password, expected_pid=self.process.pid,
            expected_directory=self.root / 'data', max_scan_calls=100, max_keys=1000, batch_size=50, timeout=5)
        self.assertEqual(removed, {'sessions_removed': 2, 'challenges_removed': 1})
        remaining = {k:v for k,v in original.items() if not k.startswith(('identity:session:', 'identity:challenge:'))}
        self.assertTrue(redis_snapshot(self) == remaining, 'invalidation changed retained controls')
        self.assertEqual(self.cli('CONFIG', 'SET', 'appendonly', 'yes'), 'OK')
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            facts = self.server_info('persistence')
            if facts['aof_enabled']=='1' and facts['aof_rewrite_in_progress']=='0' and facts['aof_rewrite_scheduled']=='0' and facts['aof_last_bgrewrite_status']=='ok':
                break
            time.sleep(0.05)
        else:
            self.fail('sanitized Redis did not produce durable AOF')
        self.stop()
        shutil.copyfile(backup / 'redis.rdb', self.root / 'data/redis.rdb')
        self.start(True)
        self.assertTrue(redis_snapshot(self) == remaining, 'old RDB revived invalidated credentials')
        self.config['redis_port'] = self.port
        (self.root / 'config/settings.json').write_text(json.dumps(self.config))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--release', required=True, type=Path)
    parser.add_argument('--postgres-bin', required=True, type=Path)
    parser.add_argument('--redis-bin', required=True, type=Path)
    parser.add_argument('--result', required=True, type=Path)
    args = parser.parse_args()
    os.umask(0o077)
    os.environ['PATH'] = str(args.postgres_bin.resolve()) + ':' + str(args.redis_bin.resolve()) + ':' + os.environ['PATH']
    PopulatedRestore.release = args.release.resolve(strict=True)
    PopulatedRestore.tools = {name: (args.postgres_bin / name).resolve(strict=True)
                             for name in ('postgres', 'psql', 'pg_dump', 'pg_restore')}
    PopulatedRestore.tools.update({key:(args.redis_bin / name).resolve(strict=True) for key,name in
                                   [('redis_server','redis-server'), ('redis_cli','redis-cli'), ('redis_check_rdb','redis-check-rdb')]})
    PopulatedRestore.result = {'status':'not_passed'}
    PopulatedRestore.diagnostics = args.result.parent / 'private-native-diagnostics' / str(uuid.uuid4())
    suite = unittest.TestSuite([PopulatedRestore('test_populated_capture_restore_and_authentication')])
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    PopulatedRestore.result['status'] = 'passed' if result.wasSuccessful() else 'not_passed'
    args.result.write_text(json.dumps(PopulatedRestore.result, indent=2) + '\n')
    raise SystemExit(0 if result.wasSuccessful() else 1)


if __name__ == '__main__':
    main()
