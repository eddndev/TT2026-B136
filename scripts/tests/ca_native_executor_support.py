"""Disposable native CA and external holder; opt in with explicit binary paths."""
from contextlib import contextmanager
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
import uuid
from unittest.mock import patch

from ca_issuance_support import OPERATOR, REPOSITORY, issuance
from native_executor import NativeExecutor


class NativeFixture(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        inspector, cli = os.environ.get("CA_NATIVE_INSPECTOR"), os.environ.get("CA_NATIVE_CLI")
        if not inspector or not cli:
            raise unittest.SkipTest("native CA crypto requires CA_NATIVE_INSPECTOR and CA_NATIVE_CLI")
        cls.inspector, cls.cli = Path(inspector).resolve(), Path(cli).resolve()
        for path in (cls.inspector, cls.cli):
            if not path.is_file():
                raise AssertionError("configured native CA binary is absent")

    def command(self, args, **kwargs):
        result = subprocess.run(args, capture_output=True, timeout=45, umask=0o077, **kwargs)
        self.assertEqual(result.returncode, 0, "owned native CA fixture command failed")
        return result.stdout

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="ca-native-")
        self.addCleanup(temporary.cleanup)
        self.scratch = Path(temporary.name)
        self.root = self.scratch / "qadra"
        self.ca = self.root / "data/ca"
        (self.root / "config").mkdir(mode=0o700, parents=True)
        self.command(["bash", REPOSITORY / "pki/init-ca.sh"],
                     env={**os.environ, "PKI_CA_DIR": str(self.ca)})
        holder = self.scratch / "external-holder"
        holder.mkdir(mode=0o700)
        self.holder_key, csr = holder / "holder.key.pem", holder / "holder.csr.pem"
        self.command(["openssl", "genpkey", "-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:3072",
                      "-pkeyopt", "rsa_keygen_pubexp:65537", "-out", self.holder_key])
        self.command(["openssl", "req", "-new", "-sha256", "-key", self.holder_key, "-out", csr,
                      "-subj", "/C=MX/O=Despacho Juridico Demo/CN=External Native Holder"])
        self.csr = csr.read_bytes()
        self.root_pem = (self.ca / "ca.crt.pem").read_bytes()
        self.executor = NativeExecutor(self.inspector, self.cli, REPOSITORY / "pki")
        self.admission = self.executor.admit(self.csr, self.root_pem)
        self.lock_observations = self.scratch / "inherited-lock.txt"

    def request(self, purpose):
        return {"csr_pem": self.csr.decode("ascii"), "purpose": purpose,
                "holder": "External Native Holder", "reference": "native-owned-case",
                "criterion": "synthetic acceptance", "operator_id": OPERATOR,
                "expected_root_fingerprint": self.admission["root_fingerprint"]}

    def issue(self, purpose, *, operation=None):
        operation = operation or str(uuid.uuid4())
        return operation, issuance.issue(self.root, operation, self.request(purpose), self.executor)

    def resume(self, operation):
        return issuance.resume(self.root, operation, self.executor)

    @contextmanager
    def inherited_openssl(self):
        original = shutil.which("openssl")
        self.assertIsNotNone(original)
        directory = self.scratch / "observed-bin"
        directory.mkdir(mode=0o700)
        wrapper = directory / "openssl"
        # The wrapper observes the inherited descriptor, then execs real OpenSSL.
        body = ("#!" + str(Path(sys.executable).resolve()) + "\n"
                "import os, pathlib, sys\n"
                "expected = pathlib.Path(" + repr(str(self.root / "deploy.lock")) + ").stat()\n"
                "found = False\n"
                "for entry in pathlib.Path('/proc/self/fd').iterdir():\n"
                "    try: current = os.fstat(int(entry.name))\n"
                "    except OSError: continue\n"
                "    found = found or (current.st_dev,current.st_ino)==(expected.st_dev,expected.st_ino)\n"
                "if not found: sys.exit(72)\n"
                "with open(" + repr(str(self.lock_observations)) + ", 'a') as log: log.write('inherited\\n')\n"
                "os.execv(" + repr(original) + ", [" + repr(original) + ", *sys.argv[1:]])\n")
        wrapper.write_text(body, encoding="ascii")
        wrapper.chmod(0o700)
        with patch.dict(os.environ, {"PATH": str(directory) + os.pathsep + os.environ["PATH"]}):
            yield
