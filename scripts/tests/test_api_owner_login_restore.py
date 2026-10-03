"""Restoration evidence must distinguish invalidation from elapsed validity."""
import base64
import copy
import importlib.util
from pathlib import Path
import unittest


SOURCE = Path(__file__).resolve().parents[1] / "api_owner_login_restore.py"
SPEC = importlib.util.spec_from_file_location("api_owner_login_restore_test", SOURCE)
HELPER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HELPER)

NOW = 1_800_000_000_000
AUTH = {
    "first": "identity:certificate-login:" + "a" * 64,
    "mfa": "identity:challenge:" + "b" * 64,
    "session": "identity:session:" + "c" * 64,
}
RATE = "identity:certificate-login-rate:v1:start:global"
PASSWORD = "identity:password-failures:" + "d" * 64
REPLAY = "identity:totp-used:" + "e" * 64
RESET = "identity:password-reset:v1:request:email:" + "f" * 64
SENTINEL = "owner-login-restore:" + "1" * 32 + ":control"
CONTROLS = [RATE, PASSWORD, REPLAY, RESET, SENTINEL]


def encoded(value):
    return base64.b64encode(value).decode("ascii")


def scalar(value, expires):
    return {"type": "string", "dump_base64": encoded(value),
            "expires_at_unix_ms": expires}


def fields(values, expires):
    return {"type": "hash", "fields": [[encoded(k), encoded(v)] for k, v in values],
            "expires_at_unix_ms": expires}


def stored():
    return {
        AUTH["first"]: scalar(b"private public-capture fixture", NOW + 299_000),
        AUTH["mfa"]: scalar(b"private MFA fixture", NOW + 298_000),
        AUTH["session"]: fields([(b"version", b"2"), (b"origin", b"certificate")],
                                NOW + 3_600_000),
        RATE: fields([(b"version", b"1"), (b"count", b"4"),
                      (b"expires_at_unix_ms", str(NOW + 3_600_000).encode("ascii"))],
                     NOW + 3_600_000),
        PASSWORD: scalar(b"opaque scalar dump\x00\xff", NOW + 900_000),
        REPLAY: scalar(b"opaque replay dump", NOW + 90_000),
        RESET: fields([(b"version", b"1"), (b"count", b"1")], NOW + 3_600_000),
        SENTINEL: scalar(b"unchanged sentinel dump", -1),
    }


def absent():
    return {"type": "none", "expires_at_unix_ms": -2}


class OwnerLoginRestoreEvidence(unittest.TestCase):
    def test_live_owned_records_removed_and_exact_control_bytes_preserved(self):
        values = stored()
        reads = []

        def read(key):
            reads.append(key)
            return copy.deepcopy(values.get(key, absent()))

        snapshot = HELPER.capture_boundary(AUTH, CONTROLS, read, NOW)
        self.assertEqual(set(reads), set(AUTH.values()) | set(CONTROLS))
        for key in AUTH.values():
            del values[key]
        # Hash order is not durable data; field bytes and their deadline are.
        values[RATE]["fields"].reverse()
        values[RESET]["fields"].reverse()
        reads.clear()
        HELPER.verify_invalidation(snapshot, read, NOW + 1_000)
        self.assertEqual(set(reads), set(AUTH.values()) | set(CONTROLS))
        # A later observation must not mutate the already captured evidence.
        values[PASSWORD]["dump_base64"] = encoded(b"changed scalar")
        with self.assertRaises(AssertionError):
            HELPER.verify_invalidation(snapshot, read, NOW + 2_000)

    def test_remaining_authentication_or_changed_control_cannot_pass(self):
        original = stored()
        snapshot = HELPER.capture_boundary(
            AUTH, CONTROLS, lambda key: copy.deepcopy(original[key]), NOW)
        removed = {key: copy.deepcopy(value) for key, value in original.items()
                   if key not in AUTH.values()}
        changed = []
        for key in AUTH.values():
            value = copy.deepcopy(removed)
            value[key] = copy.deepcopy(original[key])
            changed.append(("authentication retained", value))
        value = copy.deepcopy(removed)
        value[RATE]["fields"][1][1] = encoded(b"5")
        changed.append(("budget count changed", value))
        value = copy.deepcopy(removed)
        value[RESET]["expires_at_unix_ms"] += 1
        changed.append(("absolute deadline changed", value))
        value = copy.deepcopy(removed)
        value[PASSWORD]["dump_base64"] = encoded(b"different scalar dump")
        changed.append(("scalar bytes changed", value))
        value = copy.deepcopy(removed)
        value[RATE] = scalar(b"wrong type", NOW + 3_600_000)
        changed.append(("control type changed", value))
        value = copy.deepcopy(removed)
        del value[REPLAY]
        changed.append(("replay control removed", value))
        for reason, values in changed:
            with self.subTest(reason=reason), self.assertRaises(AssertionError):
                HELPER.verify_invalidation(snapshot,
                    lambda key: copy.deepcopy(values.get(key, absent())), NOW + 1_000)

    def test_expired_or_unowned_capture_is_rejected_before_reading_restored_state(self):
        values = stored()
        snapshot = HELPER.capture_boundary(
            AUTH, CONTROLS, lambda key: copy.deepcopy(values[key]), NOW)

        def no_read(_key):
            raise RuntimeError("expired or malformed evidence must fail before Redis reads")

        # Earliest authentication expiry is exclusive; passing after it proves nothing.
        for at in [NOW + 298_000, NOW + 299_000, NOW - 1]:
            with self.subTest(at=at), self.assertRaises(AssertionError):
                HELPER.verify_invalidation(snapshot, no_read, at)
        for role, replacement in [
                ("first", "identity:certificate-login-rate:v1:start:global"),
                ("mfa", "identity:challenge:" + "b" * 64 + ":extra"),
                ("session", "identity:session:" + "C" * 64)]:
            with self.subTest(role=role), self.assertRaises(AssertionError):
                HELPER.capture_boundary({**AUTH, role: replacement}, CONTROLS, no_read, NOW)
        for invalid in [absent(), scalar(b"missing TTL", -1),
                        scalar(b"already expired", NOW),
                        {"type": "list", "expires_at_unix_ms": NOW + 10_000}]:
            values[AUTH["first"]] = invalid
            with self.subTest(kind=invalid["type"]), self.assertRaises(AssertionError):
                HELPER.capture_boundary(AUTH, CONTROLS,
                    lambda key: copy.deepcopy(values[key]), NOW)


if __name__ == "__main__":
    unittest.main()
