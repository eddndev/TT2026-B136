"""Characterize late imports across exchange, without running deployment code."""
import json
import os
from pathlib import Path
import selectors
import subprocess
import sys
import tempfile
import unittest

from deploy_controller_publication_support import exchange_fixture_directories, put

PROGRAM = r"""
import json
import os
import sys

directory, policy = sys.argv[1:]
descriptor = None
if policy == "pinned":
    descriptor = os.open(directory, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    directory = "/proc/self/fd/" + str(descriptor)
sys.path.insert(0, directory)
import generation_early
print(json.dumps({"ready": generation_early.VALUE}), flush=True)
if sys.stdin.readline(32) != "continue\n":
    raise RuntimeError("missing bounded parent continuation")
import generation_late
print(json.dumps({"early": generation_early.VALUE, "late": generation_late.VALUE}), flush=True)
if descriptor is not None:
    os.close(descriptor)
"""


def stop_owned_process(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.communicate(timeout=2)
        except subprocess.TimeoutExpired:
            process.kill()
            process.communicate(timeout=2)
    for stream in (process.stdin, process.stdout, process.stderr):
        if stream is not None:
            stream.close()


@unittest.skipUnless(sys.platform == "linux", "requires Linux directory exchange and procfs")
class ControllerImportCharacterization(unittest.TestCase):
    def test_late_path_import_mixes_generations_while_a_pinned_directory_keeps_the_fixture(self):
        for policy in ("pathname", "pinned"):
            with self.subTest(policy=policy), tempfile.TemporaryDirectory() as temporary:
                base = Path(temporary)
                installed, candidate = base / "tools", base / "candidate"
                for directory, value in ((installed, "A"), (candidate, "B")):
                    directory.mkdir(mode=0o700)
                    for name in ("generation_early.py", "generation_late.py"):
                        put(directory / name, ("VALUE = '" + value + "'\n").encode("ascii"))
                process = subprocess.Popen(
                    [sys.executable, "-I", "-B", "-u", "-c", PROGRAM, str(installed), policy],
                    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                    text=True, encoding="ascii", cwd=base,
                )
                try:
                    with selectors.DefaultSelector() as selector:
                        selector.register(process.stdout, selectors.EVENT_READ)
                        self.assertTrue(selector.select(5), "child did not reach its first import")
                        ready = process.stdout.readline(256)
                    self.assertEqual(json.loads(ready), {"ready": "A"})
                    exchange_fixture_directories(installed, candidate)
                    output, error = process.communicate("continue\n", timeout=5)
                    self.assertEqual((process.returncode, error), (0, ""))
                    self.assertLess(len(output), 256)
                    result = json.loads(output)
                    self.assertEqual(result, {"early": "A", "late": "B" if policy == "pathname" else "A"})
                    # True documents an unsafe baseline, not operational acceptance.
                    self.assertEqual(result["early"] != result["late"], policy == "pathname")
                    self.assertEqual(list(base.rglob("__pycache__")), [])
                finally:
                    stop_owned_process(process)


if __name__ == "__main__":
    unittest.main()
