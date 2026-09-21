"""A cached build must not make old coverage evidence eligible for publication."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "ci_coverage", Path(__file__).resolve().parents[1] / "ci-coverage.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class CoverageCacheTests(unittest.TestCase):
    def test_cleanup_keeps_binary_and_drops_all_previous_evidence(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name in ("old.profraw", "old.profdata", "old-profraw-list",
                         "coverage.json", "compiled-binary"):
                (root / name).write_text("old")
            MODULE.clear_measurements(root, root / "coverage.json")
            self.assertEqual([p.name for p in root.iterdir()], ["compiled-binary"])

    def test_toolchain_flags_and_target_inventory_invalidate_cache(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "Cargo.toml").write_text("initial")
            key = MODULE.cache_key(root, {}, "rustc initial")
            self.assertEqual(key, MODULE.cache_key(root, {}, "rustc initial"))
            self.assertNotEqual(key, MODULE.cache_key(root, {}, "rustc updated"))
            self.assertNotEqual(key, MODULE.cache_key(root, {"RUSTFLAGS": "-C opt-level=1"}, "rustc initial"))
            (root / "Cargo.toml").write_text("different targets")
            self.assertNotEqual(key, MODULE.cache_key(root, {}, "rustc initial"))


if __name__ == "__main__":
    unittest.main()
