"""Probe actual loopback HTTP responses without production credentials or services."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
from runtime import Runtime


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/api/v1/auth/me":
            status, kind, content = self.server.api_response
        elif self.path == "/version.json":
            status, kind, content = 200, "application/json", json.dumps(self.server.identity).encode()
        else:
            status, kind, content = 200, "text/html", b"<!doctype html><html>Qadra</html>"
        self.send_response(status)
        self.send_header("Content-Type", kind)
        self.end_headers()
        self.wfile.write(content)

    def log_message(self, *args):
        pass


class HealthTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "config").mkdir()
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.server.identity = {"version": "v1.2.3", "commit": "a" * 40}
        self.server.api_response = (401, "application/json", b'{"error":"unauthorized"}')
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.addCleanup(self.close_server)
        port = self.server.server_address[1]
        (self.root / "config/settings.json").write_text(json.dumps({"api_port": port, "web_port": port}))
        (self.root / "release.json").write_text(json.dumps(self.server.identity))
        self.runtime = Runtime(self.root)

    def close_server(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()

    def test_accepts_exact_identity_frontend_and_json_unauthorized(self):
        with patch.object(self.runtime, "dependencies") as dependencies:
            self.runtime.check(self.root)
        dependencies.assert_called_once()

    def test_rejects_wrong_deployed_commit(self):
        self.server.identity = {"version": "v1.2.3", "commit": "b" * 40}
        with patch.object(self.runtime, "dependencies"), self.assertRaisesRegex(RuntimeError, "identity"):
            self.runtime.check(self.root)

    def test_html_unauthorized_is_not_api_health(self):
        self.server.api_response = (401, "text/html", b"<html>nginx unauthorized</html>")
        with patch.object(self.runtime, "dependencies"), self.assertRaisesRegex(RuntimeError, "API"):
            self.runtime.check(self.root)

    def test_unhealthy_database_prevents_success(self):
        with patch.object(self.runtime, "dependencies", side_effect=RuntimeError("database unavailable")):
            with self.assertRaisesRegex(RuntimeError, "database"):
                self.runtime.check(self.root)


if __name__ == "__main__":
    unittest.main()
