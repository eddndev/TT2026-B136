"""Harmless loopback services and dependency clients for installer acceptance."""
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import pwd
import signal
import socket
import sys
import time

UNITS = ("qadra-web.service", "qadra-api.service", "qadra-postgres.service", "qadra-redis.service")
FIELDS = ("web_port", "api_port", "postgres_port", "redis_port")


def require(value):
    if not value:
        raise RuntimeError("disposable installation worker contract failed")


def read(path):
    raw = path.read_bytes()
    require(len(raw) <= 65536)
    return json.loads(raw)


def save(path, value):
    data = (json.dumps(value, sort_keys=True) + "\n").encode("ascii")
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    descriptor = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def validate(root):
    require(os.environ.get("TT_CONTROLLER_GATE_NATIVE") == "1"
            and pwd.getpwuid(os.getuid()).pw_name == "tt-runner"
            and os.getuid() == os.geteuid() and os.getuid() > 0
            and root.is_absolute() and root.resolve(strict=True) == root
            and root.name.startswith(".tt-controller-installation-")
            and root.stat().st_uid == os.getuid())


def identity():
    pid = os.getpid()
    row = Path(f"/proc/{pid}/stat").read_text()
    ticks = int(row[row.rfind(")") + 2:].split()[19])
    groups = [line[3:] for line in Path(f"/proc/{pid}/cgroup").read_text().splitlines()
              if line.startswith("0::")]
    require(len(groups) == 1)
    return {"pid": pid, "uid": os.getuid(), "start_ticks": ticks, "control_group": groups[0]}


class Handler(BaseHTTPRequestHandler):
    def setup(self):
        self.request.settimeout(2)
        super().setup()

    def do_GET(self):
        if self.path == "/api/v1/auth/me":
            status, kind, body = 401, "application/json", b'{"error":"unauthorized"}'
        elif self.server.unit == "qadra-web.service" and self.path == "/version.json":
            release = read(self.server.root / "current/release.json")
            status, kind, body = 200, "application/json", json.dumps(
                {key: release[key] for key in ("version", "commit")}).encode("ascii")
        elif self.server.unit == "qadra-web.service" and self.path == "/":
            status, kind, body = 200, "text/html", b"<html>Disposable installer fixture</html>"
        else:
            status, kind, body = 404, "application/json", b"{}"
        self.send_response(status)
        self.send_header("Content-Type", kind)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


def worker(root, unit):
    validate(root)
    require(unit in UNITS)
    task = identity()
    require(task["control_group"].endswith("/" + unit))
    config = read(root / "config/settings.json")
    port = config[FIELDS[UNITS.index(unit)]]
    stopped = []
    signal.signal(signal.SIGINT, lambda number, frame: stopped.append(signal.Signals(number).name))
    signal.signal(signal.SIGTERM, lambda number, frame: stopped.append(signal.Signals(number).name))
    until = time.monotonic() + 110
    http = unit in UNITS[:2]
    listener = HTTPServer(("127.0.0.1", port), Handler) if http else socket.socket()
    try:
        if http:
            listener.root, listener.unit, listener.timeout = root, unit, 0.1
        else:
            listener.bind(("127.0.0.1", port))
            listener.listen(4)
            listener.settimeout(0.1)
        save(root / "run" / f"ready-{unit}-{task['pid']}.json", {**task, "port": port})
        while not stopped and time.monotonic() < until:
            if http:
                listener.handle_request()
            else:
                try:
                    client, _ = listener.accept()
                except TimeoutError:
                    continue
                with client:
                    client.settimeout(2)
                    data = client.recv(128)
                    require(data == b"QADRA-READINESS\n")
                    client.sendall(b"1\n" if unit == UNITS[2] else b"PONG\n")
        require(bool(stopped))
    finally:
        listener.server_close() if http else listener.close()
    save(root / "run" / f"closed-{unit}-{task['pid']}.json",
         {**task, "signal": stopped[0], "status": 0})


def controller(root, entry, initial, late, arguments):
    validate(root)
    require(initial == late and initial in ("A", "B"))
    task = identity()
    unit = task["control_group"].rsplit("/", 1)[-1]
    require(unit in UNITS)
    save(root / "run" / f"generation-{initial}-{unit}-{task['pid']}.json",
         {**task, "initial": initial, "late": late, "entry": entry, "arguments": arguments})
    if entry == "runtime.py" and arguments == []:
        require(unit == UNITS[1])
        worker(root, unit)
    elif entry == "runtime.py" and arguments == ["--wait-api"]:
        require(unit == UNITS[0])
        from urllib.error import HTTPError, URLError
        from urllib.request import urlopen
        port = read(root / "config/settings.json")["api_port"]
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            try:
                with urlopen(f"http://127.0.0.1:{port}/api/v1/auth/me",
                             timeout=min(1, deadline - time.monotonic())):
                    raise RuntimeError("fixture API did not require authentication")
            except HTTPError as error:
                try:
                    require(error.code == 401 and error.headers.get("Content-Type") == "application/json")
                    return
                finally:
                    error.close()
            except URLError:
                time.sleep(min(0.025, max(0, deadline - time.monotonic())))
        raise RuntimeError("fixture API readiness expired")
    else:
        require(entry == "restore_fence.py" and unit in UNITS[2:] and arguments == [])


def probe(root, name, arguments):
    validate(root)
    config = read(root / "config/settings.json")
    if name == "psql":
        require(arguments == ["-XAt", "-v", "ON_ERROR_STOP=1", "-c", "SELECT 1"])
        require(os.environ.get("PGHOST") == "127.0.0.1"
                and os.environ.get("PGPORT") == str(config["postgres_port"]))
        port, expected = config["postgres_port"], b"1\n"
    else:
        require(name == "redis-cli" and arguments == ["-h", "127.0.0.1", "-p",
                str(config["redis_port"]), "PING"])
        port, expected = config["redis_port"], b"PONG\n"
    with socket.create_connection(("127.0.0.1", port), timeout=2) as client:
        client.sendall(b"QADRA-READINESS\n")
        require(client.recv(128) == expected)
    sys.stdout.write(expected.decode("ascii"))


if __name__ == "__main__":
    require(len(sys.argv) == 4 and sys.argv[1] == "--worker")
    worker(Path(sys.argv[2]), sys.argv[3])
