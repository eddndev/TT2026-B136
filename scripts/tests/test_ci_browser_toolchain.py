"""The pinned wasm-bindgen archive survives CDN failures and is cached by checksum."""
import hashlib
import importlib.util
import io
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest import mock
import urllib.error

SCRIPT = Path(__file__).resolve().parents[1] / "browser-crypto-toolchain.py"


def load():
    spec = importlib.util.spec_from_file_location("browser_toolchain", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def archive(name):
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w:gz") as bundle:
        data = b"#!/bin/sh\n"
        info = tarfile.TarInfo(f"{name}/wasm-bindgen")
        info.size = len(data)
        bundle.addfile(info, io.BytesIO(data))
    return buffer.getvalue()


class Response(io.BytesIO):
    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()


def gateway_timeout(url):
    return urllib.error.HTTPError(url, 504, "Gateway Time-out", {}, None)


class ToolchainTests(unittest.TestCase):
    def setUp(self):
        self.module = load()
        self.bytes = archive(self.module.NAME)
        self.module.SHA256 = hashlib.sha256(self.bytes).hexdigest()
        self.root = Path(tempfile.mkdtemp())
        self.module.ROOT = self.root
        self.sleeps = []

    def run_main(self, responses, cache=None):
        calls = []

        def urlopen(url, timeout):
            calls.append(url)
            outcome = responses.pop(0)
            if isinstance(outcome, Exception):
                raise outcome
            return Response(outcome)

        env = {"TT_BROWSER_TOOLCHAIN_CACHE": str(cache)} if cache else {}
        with mock.patch.dict(os.environ, env, clear=False), \
                mock.patch.object(self.module.urllib.request, "urlopen", urlopen), \
                mock.patch.object(self.module.time, "sleep", self.sleeps.append), \
                mock.patch("builtins.print"):
            if not cache:
                os.environ.pop("TT_BROWSER_TOOLCHAIN_CACHE", None)
            self.module.main()
        return calls

    def test_retries_server_errors_with_backoff(self):
        url = "https://example.invalid"
        calls = self.run_main([gateway_timeout(url), gateway_timeout(url), self.bytes])
        self.assertEqual(len(calls), 3)
        self.assertEqual(self.sleeps, [5, 10])
        binary = self.root / "output/browser-toolchain/wasm-bindgen"
        self.assertEqual(binary.read_bytes(), b"#!/bin/sh\n")

    def test_client_errors_and_exhausted_retries_fail(self):
        missing = urllib.error.HTTPError("u", 404, "Not Found", {}, None)
        with self.assertRaises(urllib.error.HTTPError):
            self.run_main([missing])
        self.assertEqual(self.sleeps, [])
        with self.assertRaises(urllib.error.HTTPError):
            self.run_main([gateway_timeout("u")] * self.module.ATTEMPTS)

    def test_persistent_cache_is_reused_only_with_the_pinned_checksum(self):
        cache = self.root / "cache"
        self.assertEqual(len(self.run_main([self.bytes], cache)), 1)
        self.assertTrue((cache / f"{self.module.NAME}.tar.gz").exists())
        self.assertEqual(self.run_main([], cache), [])
        (cache / f"{self.module.NAME}.tar.gz").write_bytes(b"tampered")
        self.assertEqual(len(self.run_main([self.bytes], cache)), 1)

    def test_a_download_with_another_checksum_is_refused(self):
        with self.assertRaises(SystemExit):
            self.run_main([b"not the pinned archive"])


if __name__ == "__main__":
    unittest.main()
