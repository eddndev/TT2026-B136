"""Private source generations and bounded owned children for launcher tests."""
import json
import os
from pathlib import Path
import selectors
import subprocess
import sys
import tempfile
import unittest

from deploy_controller_launcher_programs import CHILD_API
from deploy_controller_publication_support import (
    REPOSITORY, digest, encoded, exchange_fixture_directories, inventory, journal, put, tree,
)

LAUNCHER = REPOSITORY / "ops/controller_launcher.py"
ENTRIES = ("runtime.py", "release.py", "restore_fence.py", "renew_crl.py")


def close_child(process):
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


class Fixture:
    def __init__(self, case, source):
        temporary = tempfile.TemporaryDirectory()
        case.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.base.chmod(0o700)
        self.root = self.base / "deployment"
        self.root.mkdir(mode=0o700)
        self.tools = self.root / "tools"
        self.candidate = self.base / "candidate"
        self.source = source
        for directory, value in ((self.tools, "A"), (self.candidate, "B")):
            self.generation(directory, value)
        self.original = inventory(self.tools)
        self.expected = digest(encoded(self.original))
        self.inode = self.tools.stat().st_ino
        self.marker = self.base / "unapproved-execution"
        self.cwd = self.base / "untrusted-cwd"
        self.cwd.mkdir(mode=0o700)
        put(self.cwd / "early.py", b"raise RuntimeError('untrusted cwd import')\n")
        put(self.cwd / "late.py", b"raise RuntimeError('untrusted path import')\n")
        put(self.root / "deploy.lock", b"")
        put(self.root / "data/private-fixture", b"unrelated synthetic private bytes\n")
        self.protected = tree(self.root / "data")
        self.environment = {**os.environ, "PYTHONPATH": str(self.cwd),
                            "CONTROLLER_FIXTURE_MARKER": str(self.marker),
                            "CONTROLLER_FIXTURE_VALUE": "literal value $() ; with spaces"}

    def generation(self, directory, value):
        directory.mkdir(mode=0o700)
        for entry in ENTRIES:
            put(directory / entry, ("ENTRY_GENERATION = " + repr(value) + "\n" + self.source).encode("ascii"))
        for name in ("early.py", "late.py"):
            put(directory / name, ("VALUE = " + repr(value) + "\n").encode("ascii"))

    def command(self, entry="runtime.py", arguments=(), **changes):
        values = {"root": self.root, "entry": entry, "fingerprint": self.expected}
        values.update(changes)
        return [sys.executable, "-I", "-B", "-S", "-u", "-c", CHILD_API, str(LAUNCHER),
                str(values["root"]), values["entry"], values["fingerprint"], json.dumps(list(arguments))]

    def cli(self, entry, arguments):
        return [sys.executable, "-I", "-B", "-S", "-u", str(LAUNCHER), "--root", str(self.root),
                "--inventory-sha256", self.expected, "--entrypoint", entry, "--", *arguments]

    def start(self, command):
        return subprocess.Popen(command, cwd=self.cwd, env=self.environment,
                                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                text=True, encoding="ascii")

    def ready(self, case, process):
        with selectors.DefaultSelector() as selector:
            selector.register(process.stdout, selectors.EVENT_READ)
            case.assertTrue(selector.select(5), "owned child did not reach readiness")
            line = process.stdout.readline(4096)
        case.assertTrue(line.endswith("\n"), "owned child readiness was incomplete")
        return json.loads(line)

    def completed(self, case, process, continuation=None):
        output, error = process.communicate(continuation, timeout=5)
        case.assertLess(len(output), 16384)
        case.assertLess(len(error), 16384)
        return output, error

    def preserved(self, case):
        case.assertEqual(tree(self.root / "data"), self.protected)
        case.assertEqual(list(self.base.rglob("__pycache__")), [])


class LauncherTests(unittest.TestCase):
    def setUp(self):
        # Require the standalone launcher before starting an owned interpreter.
        with LAUNCHER.open("rb"):
            pass

    def assert_pinned(self, value, expected_name):
        path = Path(value)
        self.assertEqual(path.name, expected_name)
        self.assertEqual(path.parent.parent, Path("/proc/self/fd"))
        self.assertTrue(path.parent.name.isdecimal())

    def run_child(self, fixture, command):
        process = fixture.start(command)
        try:
            output, error = fixture.completed(self, process)
            return process.returncode, output, error, process.pid
        finally:
            close_child(process)
