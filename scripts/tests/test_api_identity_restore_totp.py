"""Restore authentication must not reuse a retained TOTP replay claim."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from urllib.parse import urlsplit


# RFC 6238's public SHA-1 test secret, using the application's six digits.
SECRET = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ"
CURRENT_CODE = "287082"
NEXT_CODE = "359152"
OWNER = "owner@example.com"
HELPER = "helper@example.com"
DEADLINE = "agenda@example.test"


class Response(io.BytesIO):
    def __init__(self, status, value):
        super().__init__(json.dumps(value).encode("ascii"))
        self.status = status


class RestoreBoundary:
    def __init__(self, after_wall=None, elapsed=None, reject_mfa=False):
        self.wall = 59.25
        self.monotonic = 10.0
        self.after_wall = after_wall
        self.elapsed = elapsed
        self.reject_mfa = reject_mfa
        self.waits = []
        self.requests = []
        self.challenges = {}
        self.claims = {(email, CURRENT_CODE) for email in (OWNER, HELPER, DEADLINE)}
        self.original_claims = self.claims.copy()

    def sleep(self, seconds):
        if not 0 < seconds <= 30:
            raise AssertionError("TOTP boundary wait must be positive and bounded")
        self.waits.append(seconds)
        self.wall = self.wall + seconds if self.after_wall is None else self.after_wall
        self.monotonic += seconds if self.elapsed is None else self.elapsed

    def request(self, request, timeout):
        assert timeout == 60
        path = urlsplit(request.full_url).path.removeprefix("/api/v1")
        body = json.loads(request.data) if request.data else None
        self.requests.append((request.get_method(), path, body, self.wall))
        if request.get_method() == "GET" and path == "/auth/me":
            return Response(401, {"error": "invalid_session"})
        if request.get_method() == "POST" and path == "/auth/login":
            token = "test-challenge-" + str(len(self.challenges))
            self.challenges[token] = body["email"]
            return Response(200, {"challenge_token": token, "expires_in_seconds": 300})
        if request.get_method() == "POST" and path == "/auth/mfa/totp":
            email = self.challenges.pop(body["challenge_token"])
            claim = (email, body["code"])
            expected = CURRENT_CODE if self.wall < 60 else NEXT_CODE
            if self.reject_mfa or body["code"] != expected or claim in self.claims:
                return Response(401, {"error": "mfa_rejected"})
            self.claims.add(claim)
            return Response(200, {"user": {"email": email}, "access_token": "test-" + email})
        raise AssertionError("Restore authentication used an unexpected HTTP operation")


class IdentityRestoreTotp(unittest.TestCase):
    @contextlib.contextmanager
    def helper(self, boundary, include_deadline=False):
        with tempfile.TemporaryDirectory(prefix="identity-restore-totp-") as directory:
            work = Path(directory)
            environment = {
                "TT_RESTORE_WORK": directory,
                "TT_RESTORE_BASE": "http://127.0.0.1:16380",
                "TT_RESTORE_OLD_OWNER": "test-old-owner",
                "TT_RESTORE_OLD_HELPER": "test-old-helper",
                "TT_RESTORE_OWNER_SECRET": SECRET,
                "TT_RESTORE_HELPER_SECRET": SECRET,
            }
            if include_deadline:
                (work / "deadline-login-material.json").write_text(json.dumps({
                    "litigator": {"email": DEADLINE, "password": "test-password", "secret": SECRET},
                }))
                (work / "deadlines-api-state.json").write_text(json.dumps({
                    "agenda": {"tokens": {"litigator": "test-old-agenda"}},
                    "retained": {"revision": "17"},
                }))
            with patch.dict(os.environ, environment, clear=True):
                source = Path(__file__).resolve().parents[1] / "api_identity_restore.py"
                spec = importlib.util.spec_from_file_location("identity_restore_totp_test", source)
                helper = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(helper)
                with contextlib.ExitStack() as stack:
                    stack.enter_context(patch.object(helper.time, "time", lambda: boundary.wall))
                    stack.enter_context(patch.object(helper.time, "monotonic", lambda: boundary.monotonic))
                    stack.enter_context(patch.object(helper.time, "sleep", boundary.sleep))
                    stack.enter_context(patch.object(helper, "urlopen", boundary.request))
                    redis = stack.enter_context(patch.object(helper, "redis", side_effect=AssertionError(
                        "Fresh authentication must not alter replay claims")))
                    stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
                    yield helper, work, redis

    def test_restore_waits_once_before_new_challenges_and_preserves_replay_claims(self):
        boundary = RestoreBoundary()
        with self.helper(boundary, include_deadline=True) as (helper, work, redis):
            helper.login()
            self.assertEqual(len(boundary.waits), 1)
            posts = [call for call in boundary.requests if call[0] == "POST"]
            self.assertEqual([call[1] for call in posts], ["/auth/login", "/auth/mfa/totp"] * 3)
            self.assertTrue(all(60 <= call[3] < 90 for call in posts))
            self.assertEqual([call[2]["code"] for call in posts if call[1] == "/auth/mfa/totp"],
                             [NEXT_CODE] * 3)
            self.assertTrue(boundary.original_claims <= boundary.claims)
            self.assertEqual(len(boundary.claims), 6)
            redis.assert_not_called()
            saved = json.loads((work / "restored-identity.json").read_text())
            self.assertEqual(saved, {"owner": "test-" + OWNER, "helper": "test-" + HELPER})
            agenda = json.loads((work / "deadlines-api-state.json").read_text())
            self.assertEqual(agenda["agenda"]["tokens"]["litigator"], "test-" + DEADLINE)
            self.assertEqual(agenda["retained"], {"revision": "17"})

    def test_clock_discontinuity_or_unbounded_wait_fails_before_any_new_challenge(self):
        cases = (
            {"after_wall": 29.25},
            {"after_wall": 59.25},
            {"after_wall": 120.0},
            {"elapsed": -1.0},
            {"elapsed": 31.001},
        )
        for options in cases:
            with self.subTest(options=options):
                boundary = RestoreBoundary(**options)
                with self.helper(boundary) as (helper, work, redis):
                    with self.assertRaises((AssertionError, RuntimeError)):
                        helper.login()
                    self.assertEqual(len(boundary.waits), 1)
                    self.assertFalse(any(call[0] == "POST" for call in boundary.requests))
                    self.assertEqual(boundary.claims, boundary.original_claims)
                    self.assertFalse((work / "restored-identity.json").exists())
                    redis.assert_not_called()

    def test_mfa_rejection_is_not_retried_or_replaced_with_recovery(self):
        boundary = RestoreBoundary(reject_mfa=True)
        with self.helper(boundary) as (helper, work, redis):
            with self.assertRaises(AssertionError):
                helper.login()
            posts = [call for call in boundary.requests if call[0] == "POST"]
            self.assertEqual([call[1] for call in posts], ["/auth/login", "/auth/mfa/totp"])
            self.assertEqual(boundary.claims, boundary.original_claims)
            self.assertFalse((work / "restored-identity.json").exists())
            redis.assert_not_called()


if __name__ == "__main__":
    unittest.main()
