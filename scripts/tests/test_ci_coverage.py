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


    def test_prune_keeps_recent_caches_and_removes_old_siblings(self):
        import os, time
        with tempfile.TemporaryDirectory() as temporary:
            parent = Path(temporary) / "coverage"
            now = time.time()
            ages = {"current": None, "recent": 3600, "old1": 3 * 86400, "old2": 4 * 86400,
                    "old3": 5 * 86400, "busy": 2 * 3600}
            for name, age in ages.items():
                (parent / name).mkdir(parents=True)
                if age is not None:
                    os.utime(parent / name, (now - age, now - age))
            (parent / "not-a-directory").write_text("x")
            removed = MODULE.prune_caches(parent / "current", keep=3, now=now)
            self.assertEqual(sorted(removed), ["old1", "old2", "old3"])
            self.assertEqual(sorted(path.name for path in parent.iterdir()),
                             ["busy", "current", "not-a-directory", "recent"])

    def test_prune_never_removes_caches_used_within_six_hours(self):
        import os, time
        with tempfile.TemporaryDirectory() as temporary:
            parent = Path(temporary) / "coverage"
            now = time.time()
            for index in range(6):
                (parent / f"active{index}").mkdir(parents=True)
                os.utime(parent / f"active{index}", (now - 60 * index, now - 60 * index))
            (parent / "current").mkdir()
            self.assertEqual(MODULE.prune_caches(parent / "current", keep=3, now=now), [])
            self.assertEqual(len(list(parent.iterdir())), 7)

    def test_prune_ignores_symbolic_links(self):
        import os, time
        with tempfile.TemporaryDirectory() as temporary:
            parent = Path(temporary) / "coverage"
            outside = Path(temporary) / "outside"
            outside.mkdir()
            (parent / "current").mkdir(parents=True)
            now = time.time()
            for index in range(4):
                (parent / f"old{index}").mkdir()
                os.utime(parent / f"old{index}", (now - 86400 * (index + 2),) * 2)
            (parent / "link").symlink_to(outside)
            MODULE.prune_caches(parent / "current", keep=2, now=now)
            self.assertTrue(outside.exists())
            self.assertTrue((parent / "link").is_symlink())


if __name__ == "__main__":
    unittest.main()
