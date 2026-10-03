"""The pinned bootstrap forwards a distinct deployment root without importing it."""
import json
from pathlib import Path
import unittest

from deploy_controller_launcher_programs import ARGUMENTS
from deploy_controller_launcher_support import (
    ENTRIES, Fixture, LauncherTests, digest, encoded, inventory, put, tree,
)

ENTRY = "controller_installation.py"
OPERATION = "10000000-0000-4000-8000-000000000001"
BOOTSTRAP = '''import json
import os
from pathlib import Path
import sys
import early


def main():
    Path(os.environ["CONTROLLER_FIXTURE_MARKER"]).write_text("executed\\n")
    import late
    print(json.dumps({"argv": sys.argv, "file": __file__, "path": sys.path,
                      "early": early.VALUE, "late": late.VALUE,
                      "early_file": early.__file__, "late_file": late.__file__,
                      "cwd": os.getcwd(), "bytecode_disabled": sys.dont_write_bytecode}))


main()
'''


class ControllerBootstrapTests(LauncherTests):
    def bootstrap(self):
        fixture = Fixture(self, ARGUMENTS)
        put(fixture.tools / ENTRY, BOOTSTRAP.encode("ascii"))
        fixture.original = inventory(fixture.tools)
        fixture.expected = digest(encoded(fixture.original))
        deployment = fixture.base / "managed-deployment"
        deployment.mkdir(mode=0o700)
        for name in ("early.py", "late.py", ENTRY):
            put(deployment / "tools" / name,
                b"raise RuntimeError('deployment controller must not run')\n")
        put(deployment / "data/private-fixture", b"preserved deployment data\n")
        intent = fixture.base / "install intent.json"
        reopen = fixture.base / "reopen authorization.json"
        put(intent, b"synthetic intent bytes for argument transport\n")
        put(reopen, b"synthetic reopen bytes for argument transport\n")
        arguments = ["--root", str(deployment), "--operation-id", OPERATION,
                     "--intent", str(intent), "--intent-sha256", digest(intent.read_bytes()),
                     "--reopen", str(reopen), "--reopen-sha256", digest(reopen.read_bytes()),
                     "--timeout", "600"]
        return fixture, deployment, arguments

    def test_installation_entry_uses_bootstrap_sources_and_forwards_the_deployment_root(self):
        fixture, deployment, arguments = self.bootstrap()
        before = tree(deployment)
        self.assertNotEqual(fixture.root, deployment)
        self.assertTrue(set(ENTRIES).issubset(fixture.original))
        code, output, error, _pid = self.run_child(fixture, fixture.cli(ENTRY, arguments))
        self.assertEqual((code, error), (0, ""))
        result = json.loads(output)
        self.assertEqual(result["argv"], [result["file"], *arguments])
        self.assertEqual((result["early"], result["late"]), ("A", "A"))
        for field, name in (("file", ENTRY), ("early_file", "early.py"), ("late_file", "late.py")):
            self.assert_pinned(result[field], name)
        self.assertEqual(result["path"][0], str(Path(result["file"]).parent))
        for path in (fixture.tools, deployment / "tools", fixture.cwd):
            self.assertNotIn(str(path), result["path"])
        self.assertNotIn("", result["path"])
        self.assertEqual(result["cwd"], str(fixture.cwd))
        self.assertTrue(result["bytecode_disabled"])
        self.assertTrue(fixture.marker.exists())
        self.assertEqual(inventory(fixture.tools), fixture.original)
        self.assertEqual(tree(deployment), before)
        fixture.preserved(self)

    def test_wrong_bootstrap_inventory_rejects_before_the_installation_entry(self):
        fixture, deployment, arguments = self.bootstrap()
        before = tree(deployment)
        fixture.expected = "0" * 64
        code, output, _error, _pid = self.run_child(fixture, fixture.cli(ENTRY, arguments))
        self.assertNotEqual(code, 0)
        self.assertEqual(output, "")
        self.assertFalse(fixture.marker.exists())
        self.assertEqual(inventory(fixture.tools), fixture.original)
        self.assertEqual(tree(deployment), before)
        fixture.preserved(self)


if __name__ == "__main__":
    unittest.main()
