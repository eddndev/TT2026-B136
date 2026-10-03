"""Exercise the SSH delivery shell with local recording executables only."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


REPOSITORY = Path(__file__).resolve().parents[2]
COMMIT = "a" * 40
INVENTORY = "b" * 64


class SshControllerTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        (self.root / "ops").symlink_to(REPOSITORY / "ops", target_is_directory=True)
        binary = self.root / "bin"
        binary.mkdir()
        self.calls = self.root / "calls.jsonl"
        recorder = ("#!/usr/bin/env python3\nimport json, os, pathlib, sys\n"
                    "name = pathlib.Path(sys.argv[0]).name\n"
                    "with open(os.environ['FIXTURE_CALLS'], 'a') as output:\n"
                    "    output.write(json.dumps([name, *sys.argv[1:]]) + '\\n')\n"
                    "if name == 'openssl': print('c' * 32)\n"
                    "if name == 'ssh-keygen': print('ssh-ed25519 fixture-public-key')\n"
                    "if name == 'ssh' and os.environ.get('FIXTURE_PREFLIGHT_FAIL') == '1':\n"
                    "    sys.exit(37)\n")
        for name in ("ssh", "scp", "openssl", "ssh-keygen"):
            path = binary / name
            path.write_text(recorder)
            path.chmod(0o700)
        release = self.root / "output/releases"
        release.mkdir(parents=True)
        artifact = "qadra-v1.2.3-" + COMMIT + ".tar.gz"
        payload = b"synthetic package, never transmitted\n"
        (release / artifact).write_bytes(payload)
        self.checksum = hashlib.sha256(payload).hexdigest()
        (release / (artifact + ".sha256")).write_text(self.checksum + "  " + artifact + "\n")
        self.environment = {**os.environ, "PATH": str(binary) + os.pathsep + os.defpath,
            "FIXTURE_CALLS": str(self.calls), "RUNNER_TEMP": str(self.root),
            "VERSION": "v1.2.3", "SOURCE_COMMIT": COMMIT, "DEPLOY_HOST": "ci.invalid",
            "DEPLOY_PORT": "22022", "DEPLOY_USER": "qadra", "DEPLOY_ROOT": "/home/qadra/qadra",
            "DEPLOY_SSH_KEY": "synthetic-private-key", "DEPLOY_KNOWN_HOSTS": "synthetic-host-key",
            "DEPLOY_CONTROLLER_SHA256": INVENTORY,
            "GITHUB_STEP_SUMMARY": str(self.root / "summary")}

    def invoke(self, **changes):
        environment = {**self.environment, **changes}
        environment = {key: value for key, value in environment.items() if value is not None}
        return subprocess.run(["bash", str(REPOSITORY / "scripts/deploy-via-ssh.sh")],
                              cwd=self.root, env=environment, capture_output=True, text=True, timeout=10)

    def recorded(self):
        return [json.loads(line) for line in self.calls.read_text().splitlines()] if self.calls.exists() else []

    def test_external_inventory_is_required_and_validated_before_ssh_material(self):
        for value in (None, "", "B" * 64, "b" * 63, "b" * 64 + ";false"):
            with self.subTest(value=value):
                result = self.invoke(DEPLOY_CONTROLLER_SHA256=value)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.recorded(), [])
                self.assertEqual(list(self.root.glob("qadra-ssh.*")), [])
                self.assertNotIn("synthetic-private-key", result.stdout + result.stderr)

    def test_activation_uses_isolated_launcher_and_exact_external_inventory(self):
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.recorded()
        ssh = [call for call in calls if call[0] == "ssh"]
        self.assertEqual(len(ssh), 2)
        self.assertIn("test -f '/home/qadra/qadra/controller_launcher.py'", ssh[0][-1])
        self.assertEqual(ssh[1][-1], "umask 077; /usr/bin/python3 -I -B -S "
            "'/home/qadra/qadra/controller_launcher.py' --root '/home/qadra/qadra' "
            "--inventory-sha256 '" + INVENTORY + "' --entrypoint release.py -- "
            "--root '/home/qadra/qadra' activate '/home/qadra/qadra/incoming/" + "c" * 32
            + ".tar.gz' '" + self.checksum + "' 'v1.2.3' '" + COMMIT + "'")
        self.assertEqual(sum(call[0] == "scp" for call in calls), 1)
        self.assertNotIn("tools/release.py", "\n".join(call[-1] for call in ssh))
        self.assertEqual(list(self.root.glob("qadra-ssh.*")), [])
        self.assertIn(COMMIT, (self.root / "summary").read_text())

    def test_failed_launcher_preflight_never_transfers_or_activates(self):
        result = self.invoke(FIXTURE_PREFLIGHT_FAIL="1")
        self.assertEqual(result.returncode, 37)
        calls = self.recorded()
        self.assertEqual(sum(call[0] == "ssh" for call in calls), 1)
        self.assertFalse(any(call[0] == "scp" for call in calls))
        self.assertFalse((self.root / "summary").exists())
        self.assertEqual(list(self.root.glob("qadra-ssh.*")), [])
