"""An approved complete generation supplies every controller source import."""
import json
import os
from pathlib import Path
import unittest

from deploy_controller_launcher_programs import ARGUMENTS, IDENTITY_API, LATE_IMPORT, REJECT_SENTINEL
from deploy_controller_launcher_support import (
    ENTRIES, Fixture, LauncherTests, close_child, digest, encoded,
    exchange_fixture_directories, inventory, journal, put, tree,
)


class ControllerLauncherGenerationTests(LauncherTests):
    def test_late_import_and_resolved_capture_inventory_keep_the_opened_generation(self):
        fixture = Fixture(self, LATE_IMPORT)
        process = fixture.start(fixture.command())
        try:
            self.assertEqual(fixture.ready(self, process),
                             {"ready": "A", "entry": "A", "pid": process.pid})
            exchange_fixture_directories(fixture.tools, fixture.candidate)
            output, error = fixture.completed(self, process, "continue\n")
            self.assertEqual((process.returncode, error), (0, ""))
            result = json.loads(output)
            self.assertEqual((result["entry"], result["early"], result["late"]), ("A", "A", "A"))
            for key, name in (("file", "runtime.py"), ("early_file", "early.py"),
                              ("late_file", "late.py")):
                self.assert_pinned(result[key], name)
            self.assertEqual(result["inventory"], fixture.original)
            self.assertEqual(result["resolved_inode"], fixture.inode)
            self.assertEqual(result["path"][0], str(Path(result["file"]).parent))
            self.assertNotIn(str(fixture.tools), result["path"])
            self.assertNotIn("", result["path"])
            self.assertTrue(result["bytecode_disabled"])
            self.assertEqual(inventory(fixture.candidate), fixture.original)
            self.assertNotEqual(inventory(fixture.tools), fixture.original)
            fixture.preserved(self)
        finally:
            close_child(process)

    def test_cli_preserves_arguments_environment_pid_and_cwd_without_nominal_import_fallback(self):
        fixture = Fixture(self, ARGUMENTS)
        for entry in ENTRIES:
            with self.subTest(entry=entry):
                arguments = (["--root", str(fixture.root), "status"]
                             if entry in ("release.py", "renew_crl.py") else [str(fixture.root)])
                if entry == "runtime.py":
                    arguments += ["--wait-api"]
                arguments += ["literal space", "$(not-a-command)", ";", "line\nbreak", "", "--"]
                with journal.locked(fixture.root):
                    code, output, error, pid = self.run_child(fixture, fixture.cli(entry, arguments))
                self.assertEqual((code, error), (0, ""))
                result = json.loads(output)
                self.assert_pinned(result["file"], entry)
                self.assertEqual(result["argv"], [result["file"], *arguments])
                self.assertEqual(result["pid"], pid)
                self.assertEqual(result["cwd"], str(fixture.cwd))
                self.assertEqual(result["environment"], fixture.environment["CONTROLLER_FIXTURE_VALUE"])
                self.assertEqual(result["early"], "A")
                self.assertEqual(result["path"][0], str(Path(result["file"]).parent))
                self.assertNotIn(str(fixture.tools), result["path"])
                self.assertNotIn(str(fixture.cwd), result["path"])
                self.assertNotIn("", result["path"])
                self.assertTrue(result["bytecode_disabled"])
                fixture.preserved(self)

    def test_unapproved_inventory_links_modes_and_entrypoints_reject_before_any_source_runs(self):
        cases = ("hash", "hash_syntax", "root_link", "tools_link", "root_mode", "tools_mode",
                 "entry_link", "entry_hardlink", "entry_fifo", "entry_mode", "late_mode", "extra", "missing",
                 "bytecode", "directory", "too_many", "too_large", "total", "entry_escape",
                 "entry_unapproved", "missing_cli_hash", "empty_source", "effective_uid",
                 "effective_gid", "foreign_owner")
        for name in cases:
            with self.subTest(name=name):
                fixture = Fixture(self, REJECT_SENTINEL)
                changes = {}
                entry = fixture.tools / "runtime.py"
                if name == "hash":
                    changes["fingerprint"] = "0" * 64
                elif name == "hash_syntax":
                    changes["fingerprint"] = "not-a-hash"
                elif name == "root_link":
                    alias = fixture.base / "alias"
                    alias.symlink_to(fixture.root, target_is_directory=True)
                    changes["root"] = alias
                elif name == "tools_link":
                    moved = fixture.base / "redirected"
                    fixture.tools.rename(moved)
                    fixture.tools.symlink_to(moved, target_is_directory=True)
                elif name == "root_mode":
                    fixture.root.chmod(0o755)
                elif name == "tools_mode":
                    fixture.tools.chmod(0o755)
                elif name == "entry_link":
                    external = fixture.base / "external.py"
                    put(external, entry.read_bytes())
                    entry.unlink()
                    entry.symlink_to(external)
                elif name == "entry_fifo":
                    entry.unlink()
                    os.mkfifo(entry, 0o600)
                elif name == "entry_hardlink":
                    os.link(entry, fixture.base / "external.py")
                elif name == "entry_mode":
                    entry.chmod(0o644)
                elif name == "late_mode":
                    (fixture.tools / "late.py").chmod(0o644)
                elif name == "extra":
                    put(fixture.tools / "unreviewed.txt", b"unapproved generation entry\n")
                elif name == "missing":
                    (fixture.tools / "late.py").unlink()
                elif name == "bytecode":
                    put(fixture.tools / "__pycache__/late.cpython-fixture.pyc", b"unapproved bytecode")
                elif name == "directory":
                    (fixture.tools / "nested").mkdir(mode=0o700)
                elif name == "too_many":
                    for index in range(65 - len(fixture.original)):
                        put(fixture.tools / f"source_{index}.py", b"VALUE = 1\n")
                    fixture.expected = digest(encoded(inventory(fixture.tools)))
                elif name == "too_large":
                    put(fixture.tools / "late.py", b"#" * (256 * 1024 + 1))
                    fixture.expected = digest(encoded(inventory(fixture.tools)))
                elif name == "total":
                    for index in range(33):
                        put(fixture.tools / f"source_{index}.py", b"#" * (256 * 1024))
                    fixture.expected = digest(encoded(inventory(fixture.tools)))
                elif name == "entry_escape":
                    changes["entry"] = "../runtime.py"
                elif name == "entry_unapproved":
                    changes["entry"] = "early.py"
                elif name == "empty_source":
                    put(fixture.tools / "late.py", b"")
                    fixture.expected = digest(encoded(inventory(fixture.tools)))
                before = tree(fixture.tools)
                command = fixture.command(**changes)
                if name == "missing_cli_hash":
                    command = fixture.cli("runtime.py", [])
                    offset = command.index("--inventory-sha256")
                    del command[offset:offset + 2]
                elif name in ("effective_uid", "effective_gid", "foreign_owner"):
                    fixture.environment["CONTROLLER_FIXTURE_IDENTITY"] = name
                    command[command.index("-c") + 1] = IDENTITY_API
                code, output, _error, _pid = self.run_child(fixture, command)
                self.assertNotEqual(code, 0, "unapproved generation was executed")
                self.assertEqual(output, "")
                self.assertFalse(fixture.marker.exists(), "a controller ran before inventory admission")
                self.assertEqual(tree(fixture.tools), before)
                self.assertEqual(tree(fixture.root / "data"), fixture.protected)


if __name__ == "__main__":
    unittest.main()
