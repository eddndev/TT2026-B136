"""Adversarial and repeatability checks for the authenticated native cache."""

import concurrent.futures
import hashlib
import importlib.util
import io
from pathlib import Path
import stat
import sys
import tempfile
import unittest
from unittest import mock
import zipfile

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
SPEC = importlib.util.spec_from_file_location("qpdf_setup", SCRIPTS / "setup-document-formats.py")
setup = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(setup)


def archive(path, extra=()):
    entries = [
        ("lib/libqpdf.so.30.4.1", stat.S_IFREG | 0o644, b"synthetic library"),
        ("lib/libqpdf.so.30", stat.S_IFLNK | 0o777, b"libqpdf.so.30.4.1"),
        ("lib/helper.so", stat.S_IFREG | 0o644, b"synthetic dependency"),
        ("bin/qpdf", stat.S_IFREG | 0o755, b"synthetic executable"),
    ] + list(extra)
    with zipfile.ZipFile(path, "w") as output:
        for name, mode, data in entries:
            info = zipfile.ZipInfo(name)
            info.create_system = 3
            info.external_attr = mode << 16
            output.writestr(info, data)
    return hashlib.sha256(path.read_bytes()).hexdigest()


class SetupTests(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        self.source = self.root / "release.zip"
        self.digest = archive(self.source)
        self.cache = self.root / "cache"
        self.patches = [
            mock.patch.object(setup, "DIGEST", self.digest),
            mock.patch.object(setup, "verify_native", side_effect=self.native),
        ]
        for patch in self.patches:
            patch.start()
            self.addCleanup(patch.stop)

    def native(self, path):
        self.assertEqual(path.read_bytes(), b"synthetic library")
        self.assertEqual(path.with_name("helper.so").read_bytes(), b"synthetic dependency")
        self.assertEqual(path.with_name("libqpdf.so.30").resolve(), path)

    def test_checksum_failure_happens_before_archive_inspection_or_execution(self):
        self.source.write_bytes(b"not a trusted zip")
        with mock.patch.object(setup, "inspect_archive") as inspect:
            with self.assertRaisesRegex(setup.SetupError, "SHA-256"):
                setup.install(self.cache, self.source)
            inspect.assert_not_called()
        setup.verify_native.assert_not_called()
        self.assertFalse(list(self.cache.glob(".staging-*")))
        self.assertFalse(list(self.cache.glob("qpdf-*")))

    def test_verified_cache_reuses_the_absolute_library_and_companion_files(self):
        result = setup.install(self.cache, self.source)
        self.assertTrue(result.is_absolute())
        self.assertEqual(result.name, "libqpdf.so.30.4.1")
        self.source.unlink()
        self.assertEqual(setup.install(self.cache, self.source), result)
        self.assertEqual(setup.verify_native.call_count, 2)

    def test_modified_or_extra_cached_files_are_quarantined_and_rebuilt(self):
        result = setup.install(self.cache, self.source)
        result.write_bytes(b"modified native code")
        result.with_name("libc.so.6").write_bytes(b"unexpected dependency")
        rebuilt = setup.install(self.cache, self.source)
        self.assertEqual(result, rebuilt)
        self.native(rebuilt)
        self.assertFalse(rebuilt.with_name("libc.so.6").exists())
        self.assertEqual(len(list(self.cache.glob(".quarantine-*"))), 1)

    def test_a_failed_native_probe_never_publishes_an_installation(self):
        setup.verify_native.side_effect = setup.SetupError("native dependency missing")
        with self.assertRaisesRegex(setup.SetupError, "native dependency"):
            setup.install(self.cache, self.source)
        self.assertFalse(list(self.cache.glob("qpdf-*")))
        self.assertFalse(list(self.cache.glob(".staging-*")))

    def test_simultaneous_installations_publish_one_complete_tree(self):
        with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
            results = list(pool.map(lambda _: setup.install(self.cache, self.source), range(4)))
        self.assertEqual(len(set(results)), 1)
        self.assertEqual(len(list(self.cache.glob("qpdf-*"))), 1)
        self.assertFalse(list(self.cache.glob(".staging-*")))
        self.assertFalse(list(self.cache.glob(".quarantine-*")))

    def test_unsupported_platform_is_reported_before_installation(self):
        with mock.patch.object(setup.platform, "system", return_value="Darwin"):
            with self.assertRaisesRegex(setup.SetupError, "Linux x86_64"):
                setup.install(self.cache, self.source)
        self.assertFalse(self.cache.exists())

    def test_archive_escape_links_special_files_and_duplicates_are_rejected(self):
        for extra in [
            ("../escape", stat.S_IFREG | 0o644, b"bad"),
            ("/lib/escape", stat.S_IFREG | 0o644, b"bad"),
            ("lib\\escape", stat.S_IFREG | 0o644, b"bad"),
            ("lib/escape", stat.S_IFLNK | 0o777, b"../../escape"),
            ("lib/escape", stat.S_IFIFO | 0o644, b""),
            ("lib/helper.so", stat.S_IFREG | 0o644, b"duplicate"),
            ("lib/libqpdf.so.30/escape", stat.S_IFREG | 0o644, b"link parent"),
        ]:
            with self.subTest(extra=extra[0]):
                bad = self.root / "bad.zip"
                digest = archive(bad, [extra])
                with mock.patch.object(setup, "DIGEST", digest):
                    with self.assertRaises(setup.SetupError):
                        setup.install(self.cache, bad)
                self.assertFalse((self.root / "escape").exists())
                self.assertFalse(list(self.cache.glob("qpdf-*")))

    def test_unresolved_or_cyclic_links_fail_as_setup_errors(self):
        for extra in [
            [("lib/missing", stat.S_IFLNK | 0o777, b"absent")],
            [("lib/a", stat.S_IFLNK | 0o777, b"b"), ("lib/b", stat.S_IFLNK | 0o777, b"a")],
        ]:
            with self.subTest(extra=extra):
                bad = self.root / "links.zip"
                digest = archive(bad, extra)
                with mock.patch.object(setup, "DIGEST", digest):
                    with self.assertRaises(setup.SetupError):
                        setup.install(self.cache, bad)
                self.assertFalse(list(self.cache.glob("qpdf-*")))

    def test_normal_execution_reserves_stdout_for_the_library_path(self):
        output, diagnostic = io.StringIO(), io.StringIO()
        with mock.patch.dict(setup.os.environ, {
            "TT_DOCUMENT_FORMATS_CACHE": str(self.cache), "TT_QPDF_ARCHIVE": str(self.source)
        }), mock.patch.object(setup.sys, "argv", ["setup-document-formats.py"]), \
                mock.patch("sys.stdout", output), mock.patch("sys.stderr", diagnostic):
            self.assertEqual(setup.main(), 0)
        self.assertEqual(output.getvalue().splitlines(), [str(next(self.cache.glob("qpdf-*/lib/libqpdf.so.30.4.1")))])
        self.assertNotEqual(diagnostic.getvalue(), "")


if __name__ == "__main__":
    unittest.main()
