"""Owned native processes for the explicit populated restoration acceptance."""
import base64
import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import time
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen
from unittest.mock import patch

from test_deployment_redis_snapshot import RedisSnapshotTests
import provision
import runtime


class NativeRestore(RedisSnapshotTests):
    release = None
    tools = None
    diagnostics = None

    def setUp(self):
        super().setUp()
        self.stop()
        self.start(True)
        self.api = None
        self.api_log = None
        self.postgres_started = False
        self.addCleanup(self.stop_owned)
        for name in ('receipts', 'run', 'data/tmp', 'releases', 'logs'):
            (self.root / name).mkdir(mode=0o700, parents=True, exist_ok=True)
        self.target = self.root / 'releases' / self.release.name
        shutil.copytree(self.release, self.target)
        (self.root / 'current').symlink_to(self.target)
        self.pg_directory = self.root / 'data/postgres'
        self.config = json.loads((self.root / 'config/settings.json').read_text())
        self.config.update(postgres_port=self.unused_port(), api_port=self.unused_port(),
                           admin_password=secrets.token_hex(24), runtime_password=secrets.token_hex(24),
                           redis_port=self.port, kek=base64.b64encode(secrets.token_bytes(32)).decode())
        self.config['redis_password'] = self.password
        (self.root / 'config/settings.json').write_text(json.dumps(self.config))
        (self.root / 'config/settings.json').chmod(0o600)
        password = self.root / 'config/initdb-password'
        password.write_text(self.config['admin_password'] + '\n')
        password.chmod(0o600)
        self.command([self.tools['postgres'].parent / 'initdb', '-D', self.pg_directory,
                      '--auth=scram-sha-256', '--no-locale', '--encoding=UTF8',
                      '--username=qadra_admin', '--pwfile', password])
        self.postgres_started = True
        self.command([self.tools['postgres'].parent / 'pg_ctl', '-D', self.pg_directory,
                      '-l', self.root / 'logs/postgres.log', '-o',
                      f"-p {self.config['postgres_port']} -h 127.0.0.1 -k {self.root}/run -c max_connections=50 -c shared_buffers=64MB", '-w', 'start'])
        self.postgres_started = True
        self.pg_pid = int((self.pg_directory / 'postmaster.pid').read_text().splitlines()[0])
        env = {**runtime.environment(self.root, admin=True), 'PGDATABASE': 'postgres'}
        statement = "CREATE ROLE qadra_runtime LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT PASSWORD '" + self.config['runtime_password'] + "'; CREATE DATABASE qadra OWNER qadra_admin;"
        self.command([self.tools['psql'], '-X', '-v', 'ON_ERROR_STOP=1'], env=env, input=statement.encode())
        with patch.object(provision, 'run', side_effect=self.command):
            provision.initialize(self.root, self.target)
        self.start_api()

    @staticmethod
    def unused_port():
        import socket
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', 0))
            return sock.getsockname()[1]

    def command(self, arguments, **kwargs):
        kwargs.pop('check', None)
        kwargs.setdefault('timeout', 120)
        kwargs.setdefault('capture_output', True)
        result = subprocess.run([str(x) for x in arguments], **kwargs)
        if result.returncode:
            # Raw backend errors stay in this fixture's private directory.
            failure = self.root / 'logs/last-command.log'
            data = result.stderr or result.stdout or b''
            failure.write_bytes(data.encode() if isinstance(data, str) else data)
            failure.chmod(0o600)
            raise AssertionError('owned native command failed; private diagnostic available')
        return result

    def start_api(self):
        env = runtime.environment(self.root)
        env.update(RUST_LOG='warn', LD_LIBRARY_PATH=str(self.target / 'lib'))
        for name in list(env):
            if name.startswith(('TT_SESSION_', 'TT_PASSWORD_RESET_', 'RESEND_')):
                env.pop(name)
        ca, tsa = self.root / 'data/ca', self.root / 'data/tsa'
        binary = self.target / 'bin/despacho-cli'
        args = [binary, 'serve', '--bind', f"127.0.0.1:{self.config['api_port']}",
                '--data-dir', self.root / 'data/legacy', '--signer-cert', ca / 'certs/qadra-server.crt.pem',
                '--signer-key', ca / 'private/qadra-server.key.pem', '--ca-cert', ca / 'ca.crt.pem',
                '--crl', ca / 'crl/crl.pem', '--tsa-config', self.target / 'pki/tsa.cnf', '--tsa-dir', tsa,
                '--qpdf-library', self.target / 'lib/libqpdf.so.30.4.1',
                '--ffmpeg-path', self.target / 'bin/ffmpeg', '--ffprobe-path', self.target / 'bin/ffprobe']
        self.api_log = (self.root / 'logs/api.log').open('ab')
        self.api = subprocess.Popen([str(x) for x in args], env=env, cwd=self.root / 'data',
                                    stdout=self.api_log, stderr=self.api_log)
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            self.assertIsNone(self.api.poll(), 'owned API exited before readiness')
            try:
                with urlopen(f"http://127.0.0.1:{self.config['api_port']}/healthz", timeout=1) as reply:
                    if reply.read(32) == b'ok':
                        return
            except (URLError, TimeoutError):
                pass
            time.sleep(0.05)
        self.fail('owned API did not become ready')

    def stop_api(self):
        if self.api is not None:
            if self.api.poll() is None:
                self.api.terminate()
                try:
                    self.api.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    self.api.kill()
                    self.api.wait(timeout=5)
                    self.fail('owned API needed forced termination')
            self.api = None
        if self.api_log is not None:
            self.api_log.close()
            self.api_log = None

    def stop_owned(self):
        try:
            self.stop_api()
        finally:
            try:
                pidfile = getattr(self, 'pg_directory', self.root / 'data/postgres') / 'postmaster.pid'
                if self.postgres_started and pidfile.exists():
                    pid = int(pidfile.read_text().splitlines()[0])
                    self.assertEqual(Path(f'/proc/{pid}/exe').resolve(strict=True), self.tools['postgres'])
                    self.command([self.tools['postgres'].parent / 'pg_ctl', '-D', self.pg_directory,
                                  '-m', 'fast', '-w', 'stop'])
                    self.postgres_started = False
            finally:
                if self.diagnostics is not None and self.result.get('status') != 'passed':
                    self.diagnostics.mkdir(mode=0o700, parents=True, exist_ok=True)
                    for path in (self.root / 'logs').glob('*.log'):
                        with path.open('rb') as stream:
                            stream.seek(max(0, path.stat().st_size - 65536))
                            target = self.diagnostics / path.name
                            target.write_bytes(stream.read(65536))
                            target.chmod(0o600)

    def request(self, path, fields=None, token=None, status=200, method=None):
        headers = {'Content-Type': 'application/json'}
        if token:
            headers['Authorization'] = 'Bearer ' + token
        body = None if fields is None else json.dumps(fields).encode()
        request = Request(f"http://127.0.0.1:{self.config['api_port']}/api/v1" + path,
                          data=body, headers=headers, method=method)
        try:
            response = urlopen(request, timeout=30)
        except HTTPError as error:
            response = error
        with response:
            self.assertEqual(response.status, status, 'native HTTP status differs')
            raw = response.read(65537)
            self.assertLessEqual(len(raw), 65536)
            return json.loads(raw) if raw else None

    def service_observation(self, arguments, **kwargs):
        self.assertEqual(str(arguments[0]), 'systemctl')
        self.assertIsNone(self.api, 'capture requires owned API stopped')
        value = str(self.process.pid) if '--property=MainPID' in arguments else 'inactive\ninactive'
        return subprocess.CompletedProcess(arguments, 0, stdout=value)

    def targets(self):
        return {'postgres': {'pid': self.pg_pid, 'data_directory': self.pg_directory,
                             'database': 'qadra', 'schema': 'public', 'owner': 'qadra_admin',
                             'runtime_role': 'qadra_runtime'},
                'redis': {'pid': self.process.pid, 'directory': self.root / 'data', 'database': 0}}
