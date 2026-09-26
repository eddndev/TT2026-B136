"""Memory selection must stay bounded even on cold or undersized runners."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location(
    "ci_resources", Path(__file__).resolve().parents[1] / "ci-resources.py"
)


class BuildBudgetTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.module = importlib.util.module_from_spec(SPEC)
        SPEC.loader.exec_module(cls.module)

    def test_room_for_two_compilers(self):
        self.assertEqual(self.module.jobs(5 * 1024**3, 512 * 1024**2), 2)

    def test_large_compiler_keeps_one_worker(self):
        self.assertEqual(self.module.jobs(5 * 1024**3, 2 * 1024**3), 1)

    def test_missing_measurement_fails_closed(self):
        self.assertEqual(self.module.jobs(5 * 1024**3, 0), 1)

    def test_tiny_budget_cannot_create_zero_workers(self):
        self.assertEqual(self.module.jobs(1024**3, 512 * 1024**2), 1)

    def test_large_machine_still_caps_at_two(self):
        self.assertEqual(self.module.jobs(64 * 1024**3, 256 * 1024**2), 2)

    def test_dedicated_runner_can_opt_into_four_compilers_with_memory_guard(self):
        self.assertEqual(self.module.jobs(24 * 1024**3, 1024**3, maximum=4), 4)
        self.assertEqual(self.module.jobs(4 * 1024**3, 1024**3, maximum=4), 1)


if __name__ == "__main__":
    unittest.main()
