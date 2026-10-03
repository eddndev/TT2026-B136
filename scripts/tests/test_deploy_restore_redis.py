"""Observe complete scans, selective removal and fail-closed invalidation receipts."""
import copy
import unittest

from deploy_restore_redis_support import RedisInvalidationFixture


class RestoreRedisTests(RedisInvalidationFixture):
    def test_only_sessions_and_challenges_are_removed_and_retry_is_empty(self):
        before = copy.deepcopy(self.redis.values)
        result = self.invoke()
        self.assertEqual(result, {"sessions_removed": 2, "challenges_removed": 1})
        preserved = {key: value for key, value in before.items()
                     if not key.startswith(("identity:session:", "identity:challenge:"))}
        self.assertTrue(self.redis.values == preserved, "values or absolute expiries changed")
        first_delete = next(index for index, call in enumerate(self.redis.calls) if call[0] == "DEL")
        initial = self.redis.calls[:first_delete]
        self.assertTrue(any(call[0] == "SCAN" and "identity:challenge:*" in call for call in initial),
                        "deletion began before both namespaces were inspected")
        last_delete = max(index for index, call in enumerate(self.redis.calls) if call[0] == "DEL")
        final = self.redis.calls[last_delete + 1:]
        for pattern in ("identity:session:*", "identity:challenge:*"):
            self.assertTrue(any(call[0] == "SCAN" and pattern in call for call in final),
                            "removal was acknowledged without an empty final scan")
        self.assertEqual(self.invoke(), {"sessions_removed": 0, "challenges_removed": 0})
        self.assertTrue(self.redis.values == preserved)

    def test_wrong_target_bad_second_namespace_and_exhausted_scan_have_no_deletions(self):
        candidates = [
            ("pid", lambda: self.redis.info_override.update(process_id="999999")),
            ("directory", lambda: setattr(self.redis, "config_directory", "/unrelated")),
            ("malformed-key", lambda: setattr(self.redis, "bad_page", ["0", ["identity:challenge:bad"]])),
            ("unfinished-scan", lambda: setattr(self.redis, "endless", True)),
            ("key-limit", lambda: self.options.update(max_keys=1)),
        ]
        for name, change in candidates:
            with self.subTest(reason=name):
                self.redis.info_override = {}
                self.redis.config_directory = str(self.root)
                self.redis.bad_page = None
                self.redis.endless = False
                self.redis.scan_calls = 0
                self.options["max_keys"] = 20
                change()
                before = copy.deepcopy(self.redis.values)
                self.rejection()
                self.assertEqual(self.redis.delete_calls, 0)
                self.assertLessEqual(self.redis.scan_calls, self.options["max_scan_calls"])
                self.assertTrue(self.redis.values == before, "preflight rejection changed keys")

    def test_key_appearing_after_delete_prevents_success_until_exact_retry(self):
        before = copy.deepcopy(self.redis.values)
        self.redis.appear_after_delete = True
        self.rejection()
        self.assertIn(self.redis.extra, self.redis.values)
        self.assertEqual(self.invoke(), {"sessions_removed": 1, "challenges_removed": 0})
        for key, value in before.items():
            if not key.startswith(("identity:session:", "identity:challenge:")):
                self.assertTrue(self.redis.values.get(key) == value, "retained state changed")

    def test_transport_failure_and_invalid_limits_never_emit_success_or_sensitive_errors(self):
        before = copy.deepcopy(self.redis.values)
        self.redis.fail_delete = True
        self.rejection()
        self.assertTrue(self.redis.values == before, "failed first removal changed state")
        self.assertEqual(self.invoke(), {"sessions_removed": 2, "challenges_removed": 1})
        for field, value in (("timeout", 0), ("timeout", float("inf")), ("max_scan_calls", 0),
                             ("max_keys", 0), ("batch_size", 0), ("max_keys", 2**63),
                             ("expected_pid", True), ("host", "198.51.100.1")):
            with self.subTest(field=field):
                calls = len(self.redis.calls)
                self.rejection(**{field: value})
                self.assertEqual(len(self.redis.calls), calls, "invalid input reached Redis")


if __name__ == "__main__":
    unittest.main()
