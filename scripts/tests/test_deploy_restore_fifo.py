"""A real FIFO cannot indefinitely block the maintenance admission reader."""
from pathlib import Path
import os
import stat
import subprocess
import sys
import unittest

from deploy_restore_fence_support import OPERATION, setup


class RestoreFifoTests(unittest.TestCase):
    def setUp(self):
        setup(self)

    def test_fifo_read_and_restore_admission_reject_within_a_bounded_process(self):
        self.marker.parent.mkdir(parents=True, mode=0o700)
        os.mkfifo(self.marker, 0o600)
        source = Path(__file__).resolve().parents[2] / "ops/deploy"
        code = """
import sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import crl_journal as journal
import restore_fence
root = Path(sys.argv[2])
try:
    journal.read(restore_fence.fence_path(root), 4096)
except ValueError:
    pass
else:
    raise SystemExit(2)
try:
    restore_fence.guard(root)
except RuntimeError:
    pass
else:
    raise SystemExit(3)
try:
    restore_fence.enter(root, sys.argv[3])
except ValueError:
    pass
else:
    raise SystemExit(4)
"""
        try:
            result = subprocess.run(
                [sys.executable, "-B", "-c", code, str(source), str(self.root), OPERATION],
                capture_output=True, timeout=2)
        finally:
            self.assertTrue(stat.S_ISFIFO(self.marker.lstat().st_mode))
        self.assertEqual((result.returncode, result.stdout, result.stderr), (0, b"", b""))


if __name__ == "__main__":
    unittest.main()
