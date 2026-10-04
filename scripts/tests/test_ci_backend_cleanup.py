"""Exercise disposable backend ownership with real Redis and workload processes.

PostgreSQL/Cargo shims isolate signal handling from migrations and compilation.
Native PostgreSQL acceptance is separate from this process-lifecycle fixture.
"""
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKLOAD = '''import json, os, re, signal, subprocess, sys
from pathlib import Path
from urllib.parse import urlparse
port = urlparse(os.environ['IDENTITY_TEST_REDIS_URL']).port
info = subprocess.check_output(['redis-cli', '-p', str(port), 'INFO', 'server'], text=True)
redis_pid = int(re.search(r'^process_id:(\\d+)', info, re.MULTILINE).group(1))
child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)'])
value = {'redis_pid': redis_pid, 'task_pid': os.getpid(), 'child_pid': child.pid, 'directory': os.environ['TT_BACKEND_TEST_DIR']}
mode = os.environ['WORKLOAD_MODE']
if mode in ('wait', 'stubborn'):
    signal.signal(signal.SIGTERM, signal.SIG_IGN if mode == 'stubborn' else lambda *args: sys.exit(0))
    signal.signal(signal.SIGINT, lambda *args: sys.exit(0))
path = Path(os.environ['OBSERVATION'])
path.with_suffix('.tmp').write_text(json.dumps(value))
path.with_suffix('.tmp').replace(path)
if mode not in ('wait', 'stubborn'):
    sys.exit(int(mode))
while True:
    signal.pause()
'''


def alive(pid):
    try:
        state = Path(f'/proc/{pid}/stat').read_text().split()[2]
        return state != 'Z'
    except FileNotFoundError:
        return False


class BackendCleanup(unittest.TestCase):
    def setUp(self):
        for name in ('redis-server', 'redis-cli'):
            self.assertIsNotNone(shutil.which(name), f'{name} is required')
        self.scratch = tempfile.TemporaryDirectory(prefix='backend-cleanup-', dir=ROOT / 'output/tmp')
        self.directory = Path(self.scratch.name)
        self.tools = self.directory / 'bin'
        self.tools.mkdir()
        for name in ('cargo', 'initdb', 'pg_ctl', 'pg_dump', 'pg_restore', 'psql'):
            path = self.tools / name
            path.write_text('#!/bin/sh\nexit 0\n')
            path.chmod(0o700)
        self.workload = self.directory / 'workload.py'
        self.workload.write_text(WORKLOAD)
        self.observation = self.directory / 'observed.json'
        self.process = None
        self.log = None
        self.sentinel = subprocess.Popen(
            [sys.executable, '-c', 'import time; time.sleep(60)'], start_new_session=True)

    def tearDown(self):
        if self.process is not None:
            if self.process.poll() is None:
                os.killpg(self.process.pid, signal.SIGTERM)
                try:
                    self.process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    os.killpg(self.process.pid, signal.SIGKILL)
                    self.process.wait(timeout=3)
            if self.observation.exists():
                value = json.loads(self.observation.read_text())
                for key in ('redis_pid', 'task_pid', 'child_pid'):
                    if alive(value[key]):
                        os.kill(value[key], signal.SIGTERM)
        if self.log is not None:
            self.log.close()
        self.sentinel.terminate()
        self.sentinel.wait(timeout=3)
        self.scratch.cleanup()

    def exercise(self, mode, expected, interrupt=None, whole_group=False):
        environment = {**os.environ, 'PATH': str(self.tools) + os.pathsep + os.environ['PATH'],
                       'TMPDIR': str(self.directory), 'TT_TEST_QPDF_LIBRARY': 'not-used-by-workload',
                       'OBSERVATION': str(self.observation), 'WORKLOAD_MODE': mode}
        self.log = (self.directory / 'run.log').open('w')
        self.process = subprocess.Popen(
            ['bash', str(ROOT / 'scripts/test-backends.sh'), sys.executable, str(self.workload)],
            cwd=ROOT, env=environment, stdout=self.log, stderr=subprocess.STDOUT,
            start_new_session=True)
        deadline = time.monotonic() + 5
        while not self.observation.exists() and self.process.poll() is None and time.monotonic() < deadline:
            time.sleep(0.02)
        self.assertTrue(self.observation.exists(), 'workload did not observe live Redis')
        value = json.loads(self.observation.read_text())
        if interrupt is not None:
            if whole_group:
                os.killpg(self.process.pid, interrupt)
            else:
                self.process.send_signal(interrupt)
        try:
            result = self.process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            self.fail('backend supervisor did not stop after its signal')
        self.assertEqual(result, expected)
        deadline = time.monotonic() + 3
        while any(alive(value[key]) for key in ('redis_pid', 'task_pid', 'child_pid')) and time.monotonic() < deadline:
            time.sleep(0.02)
        self.assertFalse(alive(value['redis_pid']), 'owned Redis survived the supervisor')
        self.assertFalse(alive(value['task_pid']), 'owned workload survived the supervisor')
        self.assertFalse(alive(value['child_pid']), 'owned descendant survived the supervisor')
        self.assertFalse(Path(value['directory']).exists(), 'owned temporary directory survived')
        self.assertIsNone(self.sentinel.poll(), 'unrelated process was terminated')

    def test_success_preserves_status_and_cleans_backends(self):
        self.exercise('0', 0)

    def test_failure_preserves_status_and_cleans_backends(self):
        self.exercise('7', 7)

    def test_term_stops_waiting_workload_and_backends(self):
        self.exercise('wait', 143, signal.SIGTERM)

    def test_int_stops_waiting_workload_and_backends(self):
        self.exercise('wait', 130, signal.SIGINT)

    def test_group_term_cleans_backends_without_affecting_unrelated_processes(self):
        self.exercise('wait', 143, signal.SIGTERM, whole_group=True)

    def test_term_escalates_for_a_workload_ignoring_the_signal(self):
        self.exercise('stubborn', 143, signal.SIGTERM)


if __name__ == '__main__':
    unittest.main()
