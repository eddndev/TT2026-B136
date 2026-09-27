import importlib.util
from pathlib import Path
import unittest
from urllib.parse import urlsplit


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "ci_test_slot", ROOT / "scripts" / "ci_test_slot.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


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
                        "TT_CI_TEST_SLOTS": "12",
                        "NEXTEST_TEST_GLOBAL_SLOT": str(slot)}) for slot in range(12)]
        for key in ("IDENTITY_TEST_DATABASE_URL", "CASE_TEST_DATABASE_URL",
                    "DOCUMENT_TEST_DATABASE_URL", "IDENTITY_TEST_REDIS_URL"):
            self.assertEqual(len({env[key] for env in environments}), 12)
        self.assertEqual(urlsplit(environments[-1]["IDENTITY_TEST_REDIS_URL"]).path, "/11")
        self.assertEqual(len({environments[0][key] for key in (
            "IDENTITY_TEST_DATABASE_URL", "CASE_TEST_DATABASE_URL",
            "DOCUMENT_TEST_DATABASE_URL")}), 3)
        parsed = urlsplit(environments[0]["CASE_TEST_DATABASE_URL"])
        self.assertEqual(parsed.netloc, "runner:p%40ss@127.0.0.1:5433")
        self.assertEqual(parsed.query, "sslmode=disable")

    def test_rejects_backend_counts_outside_the_resource_budget(self):
        for count in ("0", "13", "16", "bad"):
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


if __name__ == "__main__":
    unittest.main()
