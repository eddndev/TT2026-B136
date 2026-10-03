"""Accept redis-cli INFO output, which remains raw even with JSON selected."""
import copy
import unittest
from unittest.mock import patch

from deploy_restore_redis_support import RedisInvalidationFixture
import restore_redis


class RestoreRedisInfoTests(RedisInvalidationFixture):
    def test_raw_info_crlf_identifies_target_before_selective_removal(self):
        before = copy.deepcopy(self.redis.values)
        observed = []

        def execute(arguments, **kwargs):
            result = self.redis.execute(arguments, **kwargs)
            args = [str(value) for value in arguments]
            if "INFO" in args:
                # redis-cli emits this section as raw text even under --json.
                result.stdout = ("# Server\r\nredis_version:7.4.11\r\n"
                                 f"process_id:{self.redis.pid}\r\n"
                                 "tcp_port:16386\r\n\r\n").encode("utf-8")
                observed.append(result.stdout)
            return result

        with patch.object(restore_redis, "run", side_effect=execute):
            result = restore_redis.invalidate_sessions(**self.options)
        self.assertEqual(len(observed), 1)
        self.assertTrue(observed[0].startswith(b"# Server\r\nredis_version:7.4.11\r\n"))
        self.assertEqual(result, {"sessions_removed": 2, "challenges_removed": 1,
                                  "certificate_logins_removed": 0})
        expected = {key: value for key, value in before.items()
                    if not key.startswith(("identity:session:", "identity:challenge:",
                                           "identity:certificate-login:"))}
        self.assertTrue(self.redis.values == expected, "retained controls or expiries changed")
        first_scan = next(index for index, call in enumerate(self.redis.calls) if call[0] == "SCAN")
        self.assertEqual(self.redis.calls[:first_scan],
                         [("PING",), ("INFO", "server"), ("CONFIG", "GET", "dir")])


if __name__ == "__main__":
    unittest.main()
