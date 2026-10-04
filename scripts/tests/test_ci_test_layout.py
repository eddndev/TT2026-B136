"""Check integration source ownership through actual Cargo test targets."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SOURCE = Path(__file__).resolve().parents[1] / "check-test-layout.py"
SPEC = importlib.util.spec_from_file_location("ci_test_layout", SOURCE)
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


class TestIntegrationSourceOwnership(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="tt-test-layout-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.crate = self.root / "crates" / "sample"
        (self.crate / "tests").mkdir(parents=True)
        root_patch = patch.object(CHECKER, "ROOT", self.root)
        root_patch.start()
        self.addCleanup(root_patch.stop)

    def write(self, relative, content):
        path = self.crate / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="ascii")

    def manifest(self, targets, autotests=False):
        content = (
            '[package]\nname = "sample"\nversion = "0.1.0"\n'
            f'autotests = {str(autotests).lower()}\n'
        )
        for name, path in targets:
            content += f'\n[[test]]\nname = "{name}"\npath = "{path}"\n'
        self.write("Cargo.toml", content)

    def root_with_child(self):
        self.write("tests/root.rs", '#[path = "child.rs"]\nmod child;\n')
        self.write("tests/child.rs", "#[test]\nfn covered() {}\n")

    def test_top_level_target_owns_its_explicit_child(self):
        self.root_with_child()
        self.manifest([("root", "tests/root.rs")])
        self.assertEqual(CHECKER.check(self.crate), 1)

    def test_child_cannot_also_be_an_independent_target(self):
        self.root_with_child()
        self.manifest([("root", "tests/root.rs"), ("child", "tests/child.rs")])
        with self.assertRaisesRegex(RuntimeError, r"child\.rs: registered 2 times"):
            CHECKER.check(self.crate)

    def test_orphan_source_remains_rejected(self):
        self.write("tests/root.rs", "#[test]\nfn covered() {}\n")
        self.write("tests/orphan.rs", "#[test]\nfn forgotten() {}\n")
        self.manifest([("root", "tests/root.rs")])
        with self.assertRaisesRegex(RuntimeError, r"orphan\.rs: registered 0 times"):
            CHECKER.check(self.crate)

    def test_suite_wrapper_owns_each_direct_source_once(self):
        self.write("tests/first.rs", "#[test]\nfn first() {}\n")
        self.write("tests/second.rs", "#[test]\nfn second() {}\n")
        self.write(
            "tests/suites/root.rs",
            '#[path = "../first.rs"]\nmod first;\n'
            '#[path = "../second.rs"]\nmod second;\n',
        )
        self.manifest([("root", "tests/suites/root.rs")])
        self.assertEqual(CHECKER.check(self.crate), 1)

    def test_automatic_targets_keep_their_existing_count(self):
        self.write("tests/first.rs", "#[test]\nfn first() {}\n")
        self.write("tests/second.rs", "#[test]\nfn second() {}\n")
        self.manifest([], autotests=True)
        self.assertEqual(CHECKER.check(self.crate), 2)

    def test_suite_wrapper_owns_a_source_included_by_its_parent(self):
        self.root_with_child()
        self.write("tests/suites/suite.rs", '#[path = "../root.rs"]\nmod root;\n')
        self.manifest([("suite", "tests/suites/suite.rs")])
        self.assertEqual(CHECKER.check(self.crate), 1)

    def test_two_nested_parents_cannot_register_the_same_child(self):
        self.write("tests/first.rs", '#[path = "shared.rs"]\nmod shared;\n')
        self.write("tests/second.rs", '#[path = "shared.rs"]\nmod shared;\n')
        self.write("tests/shared.rs", "#[test]\nfn covered() {}\n")
        self.write(
            "tests/suites/suite.rs",
            '#[path = "../first.rs"]\nmod first;\n'
            '#[path = "../second.rs"]\nmod second;\n',
        )
        self.manifest([("suite", "tests/suites/suite.rs")])
        with self.assertRaisesRegex(RuntimeError, r"shared\.rs: registered 2 times"):
            CHECKER.check(self.crate)

    def test_cyclic_explicit_inclusion_is_rejected(self):
        self.root_with_child()
        self.write("tests/child.rs", '#[path = "root.rs"]\nmod root;\n')
        self.manifest([("root", "tests/root.rs")])
        with self.assertRaisesRegex(RuntimeError, "cyclic explicit module inclusion"):
            CHECKER.check(self.crate)


if __name__ == "__main__":
    unittest.main()
