"""Additional opaque authentication captures and preserved restore controls."""
import copy
import json
import subprocess

import deploy_restore_redis_support as existing

CAPTURE_PREFIX = "identity:certificate-login:"
CAPTURE_PATTERN = CAPTURE_PREFIX + "*"
PATTERNS = ("identity:session:*", "identity:challenge:*", CAPTURE_PATTERN)
CONTROL_KEYS = (
    "identity:certificate-login-rate:v1:start:global",
    "identity:certificate-login-rate:v1:start:owner-binding:" + "a" * 64,
    "identity:certificate-login-rate:v1:proof:global",
    "identity:certificate-login-rate:v1:proof:token:" + "b" * 64,
)


def budget(deadline):
    return {"version": "1", "limit": "4", "window_ms": "300000", "count": "1",
            "opened_at_unix_ms": str(deadline - 300000),
            "expires_at_unix_ms": str(deadline)}


def opaque_capture():
    # Invalidation must never decode this deliberately synthetic public value.
    return json.dumps({"version": 1, "context": {"fixture": "public capture"},
                       "statement": "opaque-statement-bytes"}, separators=(",", ":"))


def is_authentication(key):
    return key.startswith(("identity:session:", "identity:challenge:", CAPTURE_PREFIX))


class CertificateCommands(existing.RedisCommands):
    def __init__(self, owner, directory):
        super().__init__(owner, directory)
        self.values[CAPTURE_PREFIX + "a" * 64] = ("string", opaque_capture(), 2200000000010)
        self.values[CAPTURE_PREFIX + "b" * 64] = ("hash", {"corrupt": "capture"}, 2200000000011)
        for index, key in enumerate(CONTROL_KEYS):
            expiry = 2200000000020 + index
            self.values[key] = ("hash", budget(expiry), expiry)
        self.values["identity:certificate-login-other:" + "c" * 64] = ("other", 2200000000030)
        self.capture_page = None
        self.capture_endless = False
        self.fail_capture_delete = False

    def command(self, command):
        if command[0] == "SCAN" and CAPTURE_PATTERN in command:
            if self.capture_page is not None or self.capture_endless:
                self.calls.append(tuple(command))
                self.scan_calls += 1
                if self.scan_calls > 40:
                    raise AssertionError("capture scan escaped its traversal limit")
                return ["1", []] if self.capture_endless else copy.deepcopy(self.capture_page)
        if (command[0] == "DEL" and self.fail_capture_delete
                and any(key.startswith(CAPTURE_PREFIX) for key in command[1:])):
            self.calls.append(tuple(command))
            self.delete_calls += 1
            self.fail_capture_delete = False
            raise subprocess.TimeoutExpired(["redis-cli"], 1,
                                            stderr=b"private-capture-transport-sentinel")
        return super().command(command)


class CertificateInvalidationFixture(existing.RedisInvalidationFixture):
    def setUp(self):
        super().setUp()
        self.reset_commands()

    def reset_commands(self):
        self.redis = CertificateCommands(self, self.root)

    def preserved(self, values):
        return {key: value for key, value in values.items() if not is_authentication(key)}
