"""Package the current Qadra workspace without changing installed paths."""
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops" / "deploy"))
from bundle import build, extract_bundle


class WorkspaceBundleTests(unittest.TestCase):
    def test_uses_qadra_build_and_preserves_installed_bundle_contract(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            files = {
                "target/release/despacho-cli": b"binary",
                "web/apps/qadra/dist/index.html": b"current Qadra",
                "web/apps/ca/dist/index.html": b"separate CA",
                "web/dist/index.html": b"obsolete build",
                "migrations/0001.sql": b"sql",
                "pki/tsa.cnf": b"tsa",
                "native/libqpdf.so.30.4.1": b"native",
            }
            for name, data in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
            original_copy = shutil.copyfile

            def copy_public_file(source, destination, **kwargs):
                if Path(source).parent == Path("/opt/tt-media/bin"):
                    Path(destination).write_bytes(b"media fixture")
                    return destination
                return original_copy(source, destination, **kwargs)

            def checked_output(command, **kwargs):
                if command == ["git", "rev-parse", "HEAD"]:
                    return "a" * 40 + "\n"
                self.assertEqual(command, ["git", "ls-files", "pki"])
                return "pki/tsa.cnf\n"

            with patch("bundle.subprocess.check_output", side_effect=checked_output), \
                    patch("bundle.subprocess.run"), \
                    patch("bundle.shutil.copyfile", side_effect=copy_public_file):
                archive = build(root, "v0.1.7", "a" * 40,
                                root / "native/libqpdf.so.30.4.1", root / "releases")
            target = root / "installed"
            manifest = extract_bundle(archive, target, "v0.1.7", "a" * 40)
            self.assertEqual((target / "web/index.html").read_bytes(), b"current Qadra")
            self.assertEqual(json.loads((target / "web/version.json").read_text()),
                             {"version": "v0.1.7", "commit": "a" * 40})
            self.assertFalse(any("apps/" in name or "ca/" in name
                                 for name in manifest["files"]))


if __name__ == "__main__":
    unittest.main()
