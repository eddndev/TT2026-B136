"""A process death during exclusive approval publication remains recoverable."""
import os
from pathlib import Path
import subprocess
import sys
import unittest

from deploy_controller_approval_support import (
    Fixture, REJECTIONS, REPOSITORY, approver, encoded, identity, put, tree,
)
from deploy_controller_launcher_support import close_child


CHILD = r'''
import os
from pathlib import Path
import sys

source, root, operation, publication_hash, installed_hash = sys.argv[1:]
sys.path.insert(0, source)
import controller_approval as product

root = Path(root)
history = root / "maintenance/controllers" / operation / "approval.json"
real_link, real_publish = os.link, product._publish_record

def link(source, destination, *args, **kwargs):
    result = real_link(source, destination, *args, **kwargs)
    if Path(destination) == history:
        os.write(1, b"linked\n")
        os._exit(86)
    return result

def publish(path, *args, **kwargs):
    result = real_publish(path, *args, **kwargs)
    if Path(path) == history and kwargs.get("exclusive"):
        os.write(1, b"published\n")
        os._exit(86)
    return result

os.link = link
product._publish_record = publish
product.approve_controllers(root, operation,
                           expected_publication_sha256=publication_hash,
                           expected_installed_sha256=installed_hash,
                           expected_previous_approval_sha256=None)
raise RuntimeError("fixture missed exclusive publication")
'''


class ControllerApprovalCrashTests(unittest.TestCase):
    def setUp(self):
        self.product = approver()

    def test_process_death_at_exclusive_publication_resumes_without_weakening_link_guards(self):
        fixture = Fixture(self)
        foreign = fixture.operation / (".approval." + "f" * 32)
        put(foreign, b"unrelated private temporary must survive\n")
        foreign_before = tree(foreign)
        sources = fixture.sources_snapshot()
        command = [sys.executable, "-I", "-B", "-S", "-u", "-c", CHILD,
                   str(REPOSITORY / "ops/deploy"), str(fixture.root), fixture.operation_id,
                   fixture.publication_sha256, fixture.installed_sha256]
        process = subprocess.Popen(command, cwd=fixture.base, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE, text=True, encoding="ascii")
        try:
            output, error = process.communicate(timeout=5)
            self.assertEqual((process.returncode, error), (86, ""))
            self.assertIn(output, ("linked\n", "published\n"))
        finally:
            close_child(process)

        owned = identity(fixture.operation_approval)
        aliases = [path for path in fixture.operation.iterdir()
                   if path.name.startswith(".approval.") and identity(path) == owned]
        self.assertEqual(fixture.operation_approval.read_bytes(), encoded(fixture.expected()))
        self.assertEqual(fixture.operation_approval.stat().st_nlink, 2 if output == "linked\n" else 1)
        self.assertEqual(len(aliases), 1 if output == "linked\n" else 0)
        self.assertFalse(fixture.current_approval.exists())
        self.assertEqual(tree(foreign), foreign_before)
        self.assertEqual(fixture.sources_snapshot(), sources)

        receipt = fixture.approve(self.product)
        fixture.assert_approved(self, receipt)
        self.assertEqual(identity(fixture.operation_approval), owned)
        self.assertEqual(tree(foreign), foreign_before)
        self.assertEqual(fixture.sources_snapshot(), sources)

        external = fixture.base / "unowned-approval-alias"
        os.link(fixture.operation_approval, external)
        external_before = tree(external)
        selected_before = tree(fixture.current_approval)
        with self.assertRaises(REJECTIONS):
            fixture.approve(self.product)
        self.assertEqual(tree(external), external_before)
        self.assertEqual(tree(fixture.current_approval), selected_before)
        self.assertEqual(fixture.operation_approval.stat().st_nlink, 2)
        self.assertEqual(tree(foreign), foreign_before)
        fixture.assert_preserved(self)


if __name__ == "__main__":
    unittest.main()
