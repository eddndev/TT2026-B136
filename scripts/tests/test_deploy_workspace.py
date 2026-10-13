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
from test_deploy_bundle import ca_payload


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
            public_ca = ca_payload("v0.1.7")
            for name, data in public_ca.items():
                if name.startswith("ops/"):
                    files[name] = data
            files.update({"target/release/qadra-ca": b"CA binary",
                          "crates/infrastructure/src/ca_identity/schema.sql": b"independent CA schema",
                          "ops/ca/untracked.py": b"untracked local source",
                          "ops/ca/__pycache__/ignored.py": b"cached source",
                          "ops/deploy/__pycache__/ignored.pyc": b"bytecode"})
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
                if command == ["git", "ls-files", "pki"]:
                    return "pki/tsa.cnf\n"
                self.assertEqual(command, ["git", "ls-files", "-z", "ops/ca", "ops/deploy"])
                names = [name for name in public_ca if name.startswith("ops/")]
                names += ["ops/ca/__pycache__/ignored.py", "ops/deploy/__pycache__/ignored.pyc"]
                return "\0".join(names) + "\0"

            with patch("bundle.subprocess.check_output", side_effect=checked_output), \
                    patch("bundle.subprocess.run"), \
                    patch("bundle.shutil.copyfile", side_effect=copy_public_file):
                archive = build(root, "v0.1.7", "a" * 40,
                                root / "native/libqpdf.so.30.4.1", root / "releases")
                for index, missing in enumerate(("target/release/qadra-ca", "web/apps/ca/dist/index.html",
                        "crates/infrastructure/src/ca_identity/schema.sql", "ops/ca/portal.py")):
                    with self.subTest(missing=missing):
                        path = root / missing
                        saved = path.read_bytes()
                        path.unlink()
                        output = root / f"incomplete-{index}"
                        try:
                            with self.assertRaises((FileNotFoundError, ValueError)):
                                build(root, "v0.1.7", "a" * 40,
                                      root / "native/libqpdf.so.30.4.1", output)
                            self.assertEqual(list(output.glob("*.tar.gz")), [])
                        finally:
                            path.write_bytes(saved)
            target = root / "installed"
            manifest = extract_bundle(archive, target, "v0.1.7", "a" * 40)
            self.assertEqual((target / "web/index.html").read_bytes(), b"current Qadra")
            self.assertEqual(json.loads((target / "web/version.json").read_text()),
                             {"version": "v0.1.7", "commit": "a" * 40})
            self.assertFalse(any("apps/" in name for name in manifest["files"]))
            self.assertEqual((target / "bin/qadra-ca").read_bytes(), b"CA binary")
            self.assertEqual((target / "ca-web/index.html").read_bytes(), b"separate CA")
            self.assertEqual(json.loads((target / "ca-web/version.json").read_text()),
                             {"version": "v0.1.7", "commit": "a" * 40})
            self.assertEqual((target / "ca/schema.sql").read_bytes(), b"independent CA schema")
            self.assertTrue({name for name in public_ca if name.startswith("ops/")}
                            .issubset(manifest["files"]))
            self.assertFalse(any("untracked" in name or "__pycache__" in name
                                 for name in manifest["files"]))

    def test_release_script_builds_ca_after_workspace_dependency_installation(self):
        script = (Path(__file__).resolve().parents[2] / "scripts/build-release.sh").read_text()
        self.assertIn("npm --prefix web run build:ca", script)
        self.assertLess(script.index("npm --prefix web ci"), script.index("npm --prefix web run build:ca"))
        self.assertLess(script.index("npm --prefix web run build:ca"), script.index("ops/deploy/bundle.py"))


if __name__ == "__main__":
    unittest.main()
