import importlib.util
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "ci_test_shard", ROOT / "scripts" / "ci_test_shard.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class TestCiTestShard(unittest.TestCase):
    def test_every_target_runs_on_exactly_one_runner(self):
        targets = [
            ("domain", "lib", "domain"),
            ("infrastructure", "lib", "infrastructure"),
            ("infrastructure", "test", "slow"),
            ("infrastructure", "test", "medium"),
            ("infrastructure", "test", "fast"),
            ("web", "test", "http"),
        ]
        weights = {"slow": 90, "medium": 60, "fast": 30}
        first, second = MODULE.split_targets(targets, weights)
        self.assertEqual(set(first) | set(second), set(targets))
        self.assertFalse(set(first) & set(second))
        self.assertEqual((first, second), MODULE.split_targets(targets, weights))
        self.assertIn(("domain", "lib", "domain"), first)
        self.assertIn(("web", "test", "http"), first)
        self.assertTrue(any(target[0] == "infrastructure" for target in second))

    def test_limited_runner_receives_less_weighted_work(self):
        targets = [("domain", "lib", "domain")]
        targets += [("infrastructure", "test", f"group_{i}") for i in range(12)]
        weights = {f"group_{i}": 60 for i in range(12)}
        first, second = MODULE.split_targets(targets, weights)
        first_seconds = 120 + sum(MODULE.weight(target, weights) for target in first)
        second_seconds = sum(MODULE.weight(target, weights) for target in second)
        self.assertGreater(second_seconds - first_seconds, 60)

    def test_target_command_selects_one_executable(self):
        self.assertEqual(
            MODULE.command(("infrastructure", "test", "deadline_suite_1")),
            ["cargo", "llvm-cov", "--no-report", "--locked", "-p",
             "infrastructure",
             "--test", "deadline_suite_1", "--", "--test-threads=1"],
        )
        self.assertEqual(
            MODULE.command(("domain", "lib", "domain"))[-3:],
            ["-p", "domain", "--lib"],
        )
        self.assertEqual(
            MODULE.command(("despacho-cli", "bin", "despacho-cli"))[-2:],
            ["--bin", "despacho-cli"],
        )
        self.assertEqual(
            MODULE.command(("infrastructure", "lib", "infrastructure"))[-3:],
            ["--lib", "--", "--test-threads=1"],
        )


if __name__ == "__main__":
    unittest.main()
