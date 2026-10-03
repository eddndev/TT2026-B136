"""Exercise bounded restore processes without exposing backend diagnostics."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'ops/deploy'))
from restore_commands import run


def process_is_running(status):
    try:
        record = status.read_text()
    except (FileNotFoundError, ProcessLookupError):
        return False
    return record.rsplit(')', 1)[1].split()[0] != 'Z'


class RestoreCommandTests(unittest.TestCase):
    def test_success_drains_stderr_and_retains_only_stdout(self):
        result = run([sys.executable, '-c', 'import sys;print("value");print("private",file=sys.stderr)'], text=True)
        self.assertEqual(result.stdout, 'value\n')
        self.assertEqual(result.stderr, '')

    def test_bounds_both_streams_and_hides_failure_output(self):
        for source in ('import os;os.write(1,b"s"*500)',
                       'import os;os.write(2,b"s"*500)',
                       'import sys;print("private");sys.exit(7)',
                       'import os;os.write(1,b"\\xff")'):
            with self.subTest(source=source), self.assertRaisesRegex(RuntimeError, '^restore external command failed$'):
                run([sys.executable, '-c', source], maximum=128, text=True)

    def test_timeout_reaps_own_child_and_preserves_unrelated_process(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'child.pid'
            unrelated = subprocess.Popen([sys.executable, '-c', 'import time;time.sleep(30)'])
            try:
                source = ('import subprocess,sys,time;from pathlib import Path;'
                          'p=subprocess.Popen([sys.executable,"-c","import time;time.sleep(30)"]);'
                          'Path(sys.argv[1]).write_text(str(p.pid));time.sleep(30)')
                with self.assertRaisesRegex(RuntimeError, '^restore external command failed$'):
                    run([sys.executable, '-c', source, str(path)], timeout=1)
                child = int(path.read_text())
                status = Path(f'/proc/{child}/stat')
                deadline = time.monotonic() + 2
                while process_is_running(status) and time.monotonic() < deadline:
                    time.sleep(.01)
                self.assertFalse(process_is_running(status))
                self.assertIsNone(unrelated.poll())
            finally:
                unrelated.terminate()
                unrelated.wait(timeout=5)


if __name__ == '__main__':
    unittest.main()
