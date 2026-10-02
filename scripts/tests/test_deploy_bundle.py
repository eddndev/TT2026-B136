"""Release identity and archive admission must reject ambiguous input."""
import hashlib
import io
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops" / "deploy"))
from bundle import extract_bundle, validate_version


class BundleTests(unittest.TestCase):
    def test_only_canonical_three_component_versions(self):
        for value in ("v0.0.0", "v1.0.0", "v1.1.1", "v123.45.6"):
            self.assertEqual(validate_version(value), value)
        for value in ("v01.2.3", "1.2.3", "v1.2", "v1.2.3-rc1", "v1.2.3+1",
                      "v1.2.3\n", "v1.2.3/../../x", "v1.a.3", "v1.2.3.4"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                validate_version(value)

    def archive(self, root, extra=None, altered=False):
        commit = "a" * 40
        files = {"bin/despacho-cli": b"binary", "web/index.html": b"page",
                 "lib/libqpdf.so.30.4.1": b"native", "pki/tsa.cnf": b"tsa",
                 "bin/ffmpeg": b"decoder", "bin/ffprobe": b"probe",
                 "migrations/0001.sql": b"sql"}
        manifest = {"version": "v1.2.3", "commit": commit,
                    "schema": hashlib.sha256(b"0001.sql\0sql").hexdigest(),
                    "files": {k: hashlib.sha256(v).hexdigest() for k, v in files.items()}}
        files["release.json"] = json.dumps(manifest).encode()
        if altered:
            files["web/index.html"] = b"changed"
        archive = root / "release.tar.gz"
        with tarfile.open(archive, "w:gz") as tar:
            for name, data in files.items():
                member = tarfile.TarInfo(name)
                member.size = len(data)
                tar.addfile(member, io.BytesIO(data))
            if extra:
                tar.addfile(extra)
        return archive

    def test_extracts_verified_exact_version_and_commit(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            archive = self.archive(root)
            manifest = extract_bundle(archive, root / "dest", "v1.2.3", "a" * 40)
            self.assertEqual(manifest["version"], "v1.2.3")
            self.assertEqual((root / "dest/web/index.html").read_bytes(), b"page")

    def test_rejects_a_bundle_missing_either_media_tool(self):
        for program in ("ffmpeg", "ffprobe"):
            with self.subTest(program=program), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                original = self.archive(root)
                filtered = root / "without-media.tar.gz"
                with tarfile.open(original, "r:gz") as source:
                    files = {item.name: source.extractfile(item).read()
                             for item in source.getmembers()
                             if item.name != "bin/" + program}
                manifest = json.loads(files["release.json"])
                del manifest["files"]["bin/" + program]
                files["release.json"] = json.dumps(manifest).encode()
                with tarfile.open(filtered, "w:gz") as archive:
                    for name, data in files.items():
                        member = tarfile.TarInfo(name)
                        member.size = len(data)
                        archive.addfile(member, io.BytesIO(data))
                with self.assertRaisesRegex(ValueError, "inventory"):
                    extract_bundle(filtered, root / "dest", "v1.2.3", "a" * 40)

    def test_rejects_modified_payload_wrong_commit_and_extra_files(self):
        for mode in ("changed", "commit", "extra", "duplicate"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                extra = tarfile.TarInfo("extra" if mode == "extra" else "web/index.html")
                archive = self.archive(root, extra if mode in ("extra", "duplicate") else None,
                                       altered=mode == "changed")
                with self.assertRaises(ValueError):
                    extract_bundle(archive, root / "dest", "v1.2.3",
                                   "b" * 40 if mode == "commit" else "a" * 40)

    def test_rejects_path_traversal_symlinks_and_devices(self):
        for name, kind in (("../outside", tarfile.REGTYPE),
                           ("/outside", tarfile.REGTYPE), ("link", tarfile.SYMTYPE),
                           ("device", tarfile.CHRTYPE)):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                member = tarfile.TarInfo(name)
                member.type = kind
                member.linkname = "../../outside"
                with self.assertRaises(ValueError):
                    extract_bundle(self.archive(root, member), root / "dest", "v1.2.3", "a" * 40)


if __name__ == "__main__":
    unittest.main()
