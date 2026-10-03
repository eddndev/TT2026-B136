"""Disposable API restore helpers must preserve unrelated authentication state."""
import contextlib
import importlib.util
import io
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


class CertificateRestore(unittest.TestCase):
    def test_restore_invalidates_certificate_captures_but_preserves_controls(self):
        with tempfile.TemporaryDirectory(prefix="identity-restore-") as directory:
            environment = {"TT_RESTORE_WORK": directory, "TT_RESTORE_REDIS_PORT": "16379",
                           "TT_RESTORE_REDIS_PID": "4321"}
            with patch.dict(os.environ, environment, clear=True):
                source = Path(__file__).resolve().parents[1] / "api_identity_restore.py"
                spec = importlib.util.spec_from_file_location("api_identity_restore_test", source)
                helper = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(helper)
                captures = {f"identity:{kind}:" + char * 64: {"kind": kind, "expires": 17}
                            for kind, char in (("session", "a"), ("challenge", "b"),
                                               ("certificate-login", "c"))}
                retained = {"identity:certificate-login-rate:v1:proof-global": b"quota",
                            "identity:password-failures:opaque": b"password-control",
                            "identity:totp-replay:opaque": b"claim", "unrelated": b"public"}
                values = {**captures, **retained}
                commands = []

                def redis(*arguments):
                    commands.append(arguments)
                    if arguments == ("INFO", "server"):
                        return "process_id:4321\n"
                    if arguments == ("CONFIG", "GET", "dir"):
                        return "dir\n" + directory
                    if arguments[0] == "SCAN":
                        self.assertEqual(arguments[1], "0")
                        prefix = arguments[3][:-1]
                        return "\n".join(["0", *sorted(k for k in values if k.startswith(prefix))])
                    if arguments[0] == "DEL":
                        for key in arguments[1:]:
                            self.assertIn(key, captures)
                            values.pop(key)
                        return str(len(arguments) - 1)
                    self.fail("unexpected restore Redis command")

                with patch.object(helper, "redis", redis), contextlib.redirect_stdout(io.StringIO()):
                    helper.invalidate()
                self.assertEqual(values, retained, "restored first proofs must be invalidated")
                self.assertTrue(any("identity:certificate-login:*" in call for call in commands))


if __name__ == "__main__":
    unittest.main()
