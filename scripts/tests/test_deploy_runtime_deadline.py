"""Readiness preserves legacy calls and spends one explicit global deadline."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from io import BytesIO
import json
from pathlib import Path
import sys
import tempfile
import threading
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch
from urllib.error import HTTPError

from deploy_runtime_deadline_support import IDENTITY, STAGES, ReadinessIO, prepare

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
import runtime


class SlowBodyHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/api/v1/auth/me":
            status, content = 401, b'{"error":"unauthorized"}'
        elif self.path == "/version.json":
            status, content = 200, json.dumps(IDENTITY).encode()
        else:
            status, content = 200, b"<html>Qadra</html>"
        self.send_response(status)
        self.send_header("Content-Type", "application/json" if self.path != "/" else "text/html")
        self.send_header("Content-Length", str(len(content)))
        self.end_headers()
        try:
            if self.path == self.server.slow_path:
                self.server.body_started.set()
                for value in content:
                    self.wfile.write(bytes([value]))
                    self.wfile.flush()
                    if self.server.stopping.wait(0.04):
                        return
            else:
                self.wfile.write(content)
        except (BrokenPipeError, ConnectionResetError):
            pass

    def log_message(self, *args):
        pass


class RuntimeDeadlineTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        prepare(self.root)
        self.target = runtime.Runtime(self.root)

    def test_legacy_call_keeps_readiness_and_existing_per_operation_limits(self):
        with ReadinessIO(runtime, self.root) as io:
            self.assertIsNone(self.target.check(self.root))
        self.assertEqual(io.steps, list(STAGES))
        self.assertEqual(io.timeouts, [
            ("postgres", 10), ("redis", 10), ("api", 5),
            ("version_open", 5), ("frontend_open", 5), ("proxy", 5),
        ])
        self.assertTrue(all(response.closed for response in io.responses))

    def test_every_probe_uses_remaining_time_without_resetting_the_deadline(self):
        with ReadinessIO(runtime, self.root, deadline=104.0) as io:
            self.assertIsNone(self.target.check(self.root, deadline=104.0))
        self.assertEqual(io.steps, list(STAGES))
        self.assertEqual([name for name, _ in io.timeouts],
                         ["postgres", "redis", "api", "version_open", "frontend_open", "proxy"])
        self.assertTrue(all(later < earlier for (_, earlier), (_, later)
                            in zip(io.timeouts, io.timeouts[1:])))
        self.assertTrue(all(response.closed for response in io.responses))

    def test_invalid_or_exhausted_deadline_rejects_before_any_readiness_io(self):
        for deadline in (False, True, "101", float("nan"), float("inf"),
                         float("-inf"), 0, 99.0, 100.0):
            with self.subTest(deadline=repr(deadline)):
                with ReadinessIO(runtime, self.root) as io:
                    with self.assertRaises(RuntimeError):
                        self.target.check(self.root, deadline=deadline)
                self.assertEqual(io.steps, [])

    def test_valid_but_late_io_cannot_start_another_probe_or_report_ready(self):
        for stage in STAGES:
            with self.subTest(stage=stage):
                with ReadinessIO(runtime, self.root, deadline=104.0, late_at=stage) as io:
                    with self.assertRaises(RuntimeError):
                        self.target.check(self.root, deadline=104.0)
                self.assertEqual(io.steps, list(STAGES[:STAGES.index(stage) + 1]))
                self.assertTrue(all(response.closed for response in io.responses))

    def test_http_errors_close_the_response_and_stop_before_the_next_probe(self):
        for path, stage in (("/version.json", "version_open"), ("", "frontend_open")):
            with self.subTest(stage=stage):
                rejected = BytesIO(b"unavailable")
                with ReadinessIO(runtime, self.root, deadline=104.0) as io:
                    def open_response(url, **kwargs):
                        if url == "http://127.0.0.1:19082" + path:
                            io.step(stage, kwargs.get("timeout"), 5)
                            io.responses.append(rejected)
                            raise HTTPError(url, 503, "Service Unavailable", {}, rejected)
                        return io.open(url, **kwargs)

                    with patch.object(runtime, "urlopen", side_effect=open_response):
                        with self.assertRaises(RuntimeError) as failure:
                            self.target.check(self.root, deadline=104.0)
                self.assertIsInstance(failure.exception, RuntimeError)
                self.assertEqual(io.steps, list(STAGES[:STAGES.index(stage) + 1]))
                self.assertTrue(rejected.closed, "the rejected HTTP response remained open")
                self.assertTrue(all(response.closed for response in io.responses))

    def test_slow_http_body_is_bounded_by_global_budget_not_each_received_byte(self):
        for path in ("/version.json", "/"):
            with self.subTest(path=path):
                server = ThreadingHTTPServer(("127.0.0.1", 0), SlowBodyHandler)
                server.daemon_threads = True
                server.slow_path = path
                server.body_started = threading.Event()
                server.stopping = threading.Event()
                thread = threading.Thread(target=server.serve_forever, daemon=True)
                thread.start()
                self.target.config.update(api_port=server.server_port, web_port=server.server_port)

                def command(arguments, **kwargs):
                    return SimpleNamespace(stdout="1\n" if str(arguments[0]) == "psql" else "PONG\n")

                try:
                    started = time.monotonic()
                    with patch.object(runtime, "environment", return_value={}):
                        with patch.object(runtime, "run", side_effect=command):
                            with self.assertRaises(RuntimeError):
                                self.target.check(self.root, deadline=started + 0.25)
                    elapsed = time.monotonic() - started
                    self.assertTrue(server.body_started.is_set(), "the real slow-body boundary was not reached")
                    self.assertLess(elapsed, 0.55, "readiness waited beyond its global body budget")
                finally:
                    server.stopping.set()
                    server.shutdown()
                    server.server_close()
                    thread.join(timeout=2)
                    self.assertFalse(thread.is_alive())


if __name__ == "__main__":
    unittest.main()
