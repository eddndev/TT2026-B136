"""Controlled readiness I/O with an observable monotonic clock."""
from contextlib import ExitStack
from io import BytesIO
import json
from pathlib import Path
from types import SimpleNamespace
from urllib.error import HTTPError
from unittest.mock import patch


IDENTITY = {"version": "v1.2.3", "commit": "a" * 40}
STAGES = ("postgres", "redis", "api", "metadata", "version_open",
          "version_read", "frontend_open", "frontend_read", "proxy")


def prepare(root):
    (root / "config").mkdir()
    (root / "config/settings.json").write_text(json.dumps({
        "api_port": 19081, "web_port": 19082, "redis_port": 19083,
    }))
    (root / "release.json").write_text(json.dumps(IDENTITY))


class Response(BytesIO):
    def __init__(self, fixture, name, content):
        super().__init__(content)
        self.fixture = fixture
        self.name = name
        self.status = 200
        self.fp = SimpleNamespace(raw=SimpleNamespace(_sock=SimpleNamespace(settimeout=self.settimeout)))

    def settimeout(self, timeout):
        if not 0 < timeout <= min(5, self.fixture.deadline - self.fixture.now):
            raise AssertionError("body read exceeded the remaining socket budget")

    def isclosed(self):
        return self.closed

    def read(self, *args):
        self.fixture.step(self.name + "_read")
        return super().read(*args)

    def read1(self, *args):
        return self.read(*args)


class ReadinessIO:
    def __init__(self, module, root, *, deadline=None, late_at=None):
        self.module = module
        self.root = root
        self.now = 100.0
        self.deadline = deadline
        self.late_at = late_at
        self.steps = []
        self.timeouts = []
        self.responses = []
        self.previous_read = Path.read_text

    def step(self, name, timeout=None, maximum=None):
        if self.deadline is not None and self.now >= self.deadline:
            raise AssertionError("readiness began I/O after the global deadline")
        if not self.steps or self.steps[-1] != name:
            self.steps.append(name)
        if timeout is not None:
            if not isinstance(timeout, (int, float)) or not 0 < timeout <= maximum:
                raise AssertionError("readiness I/O has no valid bounded timeout")
            if self.deadline is not None and timeout > self.deadline - self.now:
                raise AssertionError("readiness reset an I/O timeout beyond the remaining budget")
            self.timeouts.append((name, timeout))
        if name == self.late_at:
            self.now = self.deadline
        else:
            self.now += 0.25

    def run(self, arguments, **kwargs):
        name = "postgres" if str(arguments[0]) == "psql" else "redis"
        self.step(name, kwargs.get("timeout"), 10)
        return SimpleNamespace(stdout="1\n" if name == "postgres" else "PONG\n")

    def open(self, url, **kwargs):
        if url.endswith("/api/v1/auth/me"):
            name = "api" if ":19081/" in url else "proxy"
            self.step(name, kwargs.get("timeout"), 5)
            response = BytesIO(b'{"error":"unauthorized"}')
            self.responses.append(response)
            raise HTTPError(url, 401, "Unauthorized", {"Content-Type": "application/json"}, response)
        name = "version" if url.endswith("/version.json") else "frontend"
        self.step(name + "_open", kwargs.get("timeout"), 5)
        content = json.dumps(IDENTITY).encode() if name == "version" else b"<html>Qadra</html>"
        response = Response(self, name, content)
        self.responses.append(response)
        return response

    def read(self, path, *args, **kwargs):
        if path == self.root / "release.json":
            self.step("metadata")
        return self.previous_read(path, *args, **kwargs)

    def __enter__(self):
        self.stack = ExitStack()
        self.stack.enter_context(patch.object(self.module.time, "monotonic", lambda: self.now))
        self.stack.enter_context(patch.object(self.module, "environment", return_value={}))
        self.stack.enter_context(patch.object(self.module, "run", side_effect=self.run))
        self.stack.enter_context(patch.object(self.module, "urlopen", side_effect=self.open))
        fixture = self

        def read(path, *args, **kwargs):
            return fixture.read(path, *args, **kwargs)

        self.stack.enter_context(patch.object(Path, "read_text", read))
        return self

    def __exit__(self, *args):
        return self.stack.__exit__(*args)
