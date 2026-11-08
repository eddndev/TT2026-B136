import importlib.util
from pathlib import Path
import sys
import unittest
from urllib.parse import urlsplit


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "ci_test_slot", ROOT / "scripts" / "ci_test_slot.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
sys.path.insert(0, str(ROOT / "scripts"))
import ci_nextest  # noqa: E402


class TestCiNextest(unittest.TestCase):
    def environment(self):
        return {
            "TT_CI_TEST_SLOTS": "4",
            "TT_CI_POSTGRES_BASE": "postgresql://runner:p%40ss@127.0.0.1:5433/postgres?sslmode=disable",
            "TT_CI_REDIS_BASE": "redis://127.0.0.1:6380/",
            "NEXTEST_TEST_GLOBAL_SLOT": "0",
        }

    def test_concurrent_slots_use_distinct_databases_and_redis_indices(self):
        environments = [MODULE.slot_environment({**self.environment(),
                        "TT_CI_TEST_SLOTS": "16",
                        "NEXTEST_TEST_GLOBAL_SLOT": str(slot)}) for slot in range(16)]
        for key in ("IDENTITY_TEST_DATABASE_URL", "CASE_TEST_DATABASE_URL",
                    "DOCUMENT_TEST_DATABASE_URL", "IDENTITY_TEST_REDIS_URL"):
            self.assertEqual(len({env[key] for env in environments}), 16)
        self.assertEqual(urlsplit(environments[-1]["IDENTITY_TEST_REDIS_URL"]).path, "/15")
        self.assertEqual(len({environments[0][key] for key in (
            "IDENTITY_TEST_DATABASE_URL", "CASE_TEST_DATABASE_URL",
            "DOCUMENT_TEST_DATABASE_URL")}), 3)
        parsed = urlsplit(environments[0]["CASE_TEST_DATABASE_URL"])
        self.assertEqual(parsed.netloc, "runner:p%40ss@127.0.0.1:5433")
        self.assertEqual(parsed.query, "sslmode=disable")

    def test_rejects_backend_counts_outside_the_resource_budget(self):
        for count in ("0", "17", "32", "bad"):
            with self.subTest(count=count), self.assertRaises(ValueError):
                MODULE.slot_environment({**self.environment(), "TT_CI_TEST_SLOTS": count})

    def test_missing_or_out_of_range_slot_fails_closed(self):
        for slot in (None, "-1", "4", "bad"):
            env = self.environment()
            if slot is None:
                del env["NEXTEST_TEST_GLOBAL_SLOT"]
            else:
                env["NEXTEST_TEST_GLOBAL_SLOT"] = slot
            with self.subTest(slot=slot), self.assertRaises(ValueError):
                MODULE.slot_environment(env)

    def test_nested_execution_does_not_append_another_suffix(self):
        env = MODULE.slot_environment(self.environment())
        self.assertEqual(MODULE.slot_environment(env), env)

    def test_requires_all_backend_addresses_instead_of_skipping_tests(self):
        for key in ("TT_CI_POSTGRES_BASE", "TT_CI_REDIS_BASE"):
            env = self.environment()
            del env[key]
            with self.assertRaises(ValueError):
                MODULE.slot_environment(env)


class TestCiNextestCommand(unittest.TestCase):
    def test_coverage_runs_instrumented_tests_without_an_intermediate_report(self):
        self.assertEqual(ci_nextest.test_command(["--workspace"], 16, coverage=True), [
            "cargo", "llvm-cov", "nextest", "--no-report", "--locked", "--workspace",
            "--profile", "ci", "--test-threads", "16"])

    def test_pull_requests_run_the_same_schedule_without_instrumentation(self):
        command = ci_nextest.test_command(["--workspace"], 16, coverage=False)
        self.assertEqual(command, [
            "cargo", "nextest", "run", "--locked", "--workspace",
            "--profile", "ci", "--test-threads", "16"])
        self.assertNotIn("llvm-cov", command)


if __name__ == "__main__":
    unittest.main()
