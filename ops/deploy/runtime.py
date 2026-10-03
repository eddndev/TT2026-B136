"""Operate private user services without changing database history on rollback."""
import json
import os
from pathlib import Path
import subprocess
import tarfile
import time
from urllib.error import HTTPError, URLError
from urllib.request import urlopen
import uuid

from runtime_readiness import read_body, remaining


def run(arguments, **kwargs):
    return subprocess.run([str(x) for x in arguments], check=True, **kwargs)


def sync_directory(path):
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def settings(root):
    return json.loads((root / "config/settings.json").read_text())


def environment(root, admin=False):
    config = settings(root)
    password = config["admin_password" if admin else "runtime_password"]
    user = "qadra_admin" if admin else "qadra_runtime"
    return {**os.environ, "RUST_LOG": "info", "PGPASSWORD": password,
            "PGHOST": "127.0.0.1", "PGPORT": str(config["postgres_port"]),
            "PGUSER": user, "PGDATABASE": "qadra", "PGCONNECT_TIMEOUT": "5",
            "DATABASE_URL": f"postgresql://{user}:{password}@127.0.0.1:{config['postgres_port']}/qadra",
            "REDIS_URL": f"redis://:{config['redis_password']}@127.0.0.1:{config['redis_port']}/0",
            "REDISCLI_AUTH": config["redis_password"], "KEK_BASE64": config["kek"],
            "PKI_CA_DIR": str(root / "data/ca"), "TSA_DIR": str(root / "data/tsa"),
            "TMPDIR": str(root / "data/tmp")}


class Runtime:
    def __init__(self, root):
        from restore_fence import guard
        guard(root)
        self.root = root
        self.config = settings(root)

    def stop(self):
        run(["systemctl", "--user", "stop", "qadra-web.service", "qadra-api.service"])

    def backup(self, *, tools=None, command=None):
        from restore_fence import guard
        guard(self.root)
        execute = run if command is None else command
        selected = {} if tools is None else tools
        state = execute(["systemctl", "--user", "show", "qadra-api.service",
                     "qadra-web.service", "--property=ActiveState", "--value"],
                    capture_output=True, text=True, timeout=15)
        if state.stdout.split() != ["inactive", "inactive"]:
            raise RuntimeError("API and web must be stopped before backup")
        redis = [selected.get("redis_cli", "redis-cli"), "-h", "127.0.0.1", "-p", str(self.config["redis_port"])]
        redis_env = {**os.environ, "REDISCLI_AUTH": self.config["redis_password"]}
        expected_pid = execute(["systemctl", "--user", "show", "qadra-redis.service",
                            "--property=MainPID", "--value"],
                           capture_output=True, text=True, timeout=15).stdout.strip()
        info = execute(redis + ["--raw", "INFO", "server"], env=redis_env,
                   capture_output=True, text=True, timeout=10).stdout
        directory = execute(redis + ["--raw", "CONFIG", "GET", "dir"], env=redis_env,
                        capture_output=True, text=True, timeout=10).stdout.splitlines()
        actual = dict(line.split(":", 1) for line in info.splitlines() if ":" in line)
        if (not expected_pid.isdecimal() or int(expected_pid) <= 0
                or actual.get("process_id") != expected_pid
                or directory != ["dir", str(self.root / "data")]):
            raise RuntimeError("Redis identity differs from the private service")
        backup = self.root / "backups" / (time.strftime("%Y%m%dT%H%M%SZ", time.gmtime()) + "-" + uuid.uuid4().hex[:8])
        backup.mkdir(mode=0o700)
        sync_directory(backup.parent)
        # Exclusive private files remain unreadable even under a permissive shell umask.
        def private_file(name):
            return os.fdopen(os.open(backup / name, os.O_CREAT | os.O_EXCL | os.O_WRONLY,
                                     0o600), "wb")

        for name in ("database.dump", "redis.rdb"):
            with private_file(name):
                pass
        execute([selected.get("pg_dump", "pg_dump"), "--format=custom", "--file", backup / "database.dump"],
            env=environment(self.root, admin=True), capture_output=True, timeout=300)
        execute(redis + ["--rdb", backup / "redis.rdb"], env=redis_env,
            capture_output=True, timeout=300)
        if any((backup / name).stat().st_size == 0 for name in ("database.dump", "redis.rdb")):
            raise RuntimeError("database or Redis backup is empty")
        execute([selected.get("redis_check_rdb", "redis-check-rdb"), backup / "redis.rdb"],
                capture_output=True, timeout=60)
        with private_file("private-state.tar.gz") as stream:
            with tarfile.open(fileobj=stream, mode="w:gz") as tar:
                for path in (self.root / "config", self.root / "data/ca", self.root / "data/tsa"):
                    if path.exists():
                        tar.add(path, arcname=path.relative_to(self.root))
        for name in ("database.dump", "redis.rdb", "private-state.tar.gz"):
            with (backup / name).open("rb") as stream:
                os.fsync(stream.fileno())
        from backup_manifest import publish
        publish(self.root, backup, controller_directory=Path(__file__).resolve().parent)
        with private_file(".complete.tmp") as marker:
            marker.write(b"database, Redis and private state captured while API stopped\n")
            marker.flush()
            os.fsync(marker.fileno())
        (backup / ".complete.tmp").rename(backup / "COMPLETE")
        try:
            sync_directory(backup)
        except OSError:
            (backup / "COMPLETE").unlink()
            sync_directory(backup)
            raise
        return backup

    def initialize(self, target):
        from restore_fence import guard
        guard(self.root)
        from provision import initialize
        initialize(self.root, target)

    def api_ready(self, *, deadline=None):
        try:
            with urlopen(f"http://127.0.0.1:{self.config['api_port']}/api/v1/auth/me",
                         timeout=remaining(deadline, 5)):
                remaining(deadline, 5)
                return False
        except HTTPError as error:
            try:
                ready = error.code == 401 and "application/json" in error.headers.get("Content-Type", "")
            finally:
                error.close()
            remaining(deadline, 5)
            return ready
        return False

    def dependencies(self, *, deadline=None):
        remaining(deadline, 10)
        env = environment(self.root)
        result = run(["psql", "-XAt", "-v", "ON_ERROR_STOP=1", "-c", "SELECT 1"],
                     env=env, capture_output=True, text=True, timeout=remaining(deadline, 10))
        remaining(deadline, 10)
        if result.stdout.strip() != "1":
            raise RuntimeError("PostgreSQL readiness failed")
        remaining(deadline, 10)
        env = environment(self.root)
        result = run(["redis-cli", "-h", "127.0.0.1", "-p", str(self.config["redis_port"]), "PING"],
                     env=env, capture_output=True, text=True, timeout=remaining(deadline, 10))
        remaining(deadline, 10)
        if result.stdout.strip() != "PONG":
            raise RuntimeError("Redis readiness failed")

    def check(self, target, *, deadline=None):
        remaining(deadline, 10)
        try:
            self._check(target, deadline)
        except HTTPError as error:
            error.close()
            if deadline is None:
                raise
            raise RuntimeError("readiness did not complete within its requested boundary") from None
        except (OSError, URLError, subprocess.SubprocessError, KeyError, TypeError, ValueError):
            if deadline is None:
                raise
            raise RuntimeError("readiness did not complete within its requested boundary") from None

    def _check(self, target, deadline):
        if deadline is None:
            self.dependencies()
            ready = self.api_ready()
        else:
            self.dependencies(deadline=deadline)
            ready = self.api_ready(deadline=deadline)
        if not ready:
            raise RuntimeError("API readiness failed")
        base = f"http://127.0.0.1:{self.config['web_port']}"
        remaining(deadline, 5)
        expected = json.loads((target / "release.json").read_text())
        remaining(deadline, 5)
        with urlopen(base + "/version.json", timeout=remaining(deadline, 5)) as response:
            remaining(deadline, 5)
            actual = (json.load(response) if deadline is None
                      else json.loads(read_body(response, 65536, deadline)))
        remaining(deadline, 5)
        if actual != {k: expected[k] for k in ("version", "commit")}:
            raise RuntimeError("served release identity differs from artifact")
        with urlopen(base, timeout=remaining(deadline, 5)) as response:
            remaining(deadline, 5)
            if response.status != 200:
                raise RuntimeError("frontend readiness failed")
            content = (response.read(1024 * 1024) if deadline is None
                       else read_body(response, 1024 * 1024, deadline))
            remaining(deadline, 5)
            if b"<html" not in content.lower():
                raise RuntimeError("frontend readiness failed")
        remaining(deadline, 5)
        try:
            with urlopen(base + "/api/v1/auth/me", timeout=remaining(deadline, 5)):
                remaining(deadline, 5)
                pass
        except HTTPError as error:
            try:
                ready = error.code == 401 and "application/json" in error.headers.get("Content-Type", "")
            finally:
                error.close()
            remaining(deadline, 5)
            if ready:
                return
        raise RuntimeError("reverse proxy readiness failed")

    def start(self, target):
        from crl_journal import guard
        guard(self.root)
        run(["systemctl", "--user", "start", "qadra-api.service"])
        self.wait_api()
        run(["systemctl", "--user", "start", "qadra-web.service"])
        for _ in range(15):
            try:
                self.check(target)
                run(["systemctl", "--user", "enable", "qadra-api.service", "qadra-web.service"])
                return
            except (OSError, URLError, RuntimeError):
                time.sleep(1)
        raise RuntimeError("frontend did not become ready")

    def wait_api(self):
        from crl_journal import guard
        guard(self.root)
        deadline = time.monotonic() + 180
        while time.monotonic() < deadline:
            guard(self.root)
            try:
                self.dependencies()
                if self.api_ready():
                    break
            except (OSError, URLError, subprocess.SubprocessError):
                pass
            time.sleep(2)
        else:
            raise RuntimeError("API did not become ready in 180 seconds")


def serve(root):
    from crl_journal import guard
    guard(root)
    target = (root / "current").resolve(strict=True)
    config = settings(root)
    env = environment(root)
    env["LD_LIBRARY_PATH"] = str(target / "lib")
    ca = root / "data/ca"
    binary = target / "bin/despacho-cli"
    args = [str(binary), "serve", "--bind", f"127.0.0.1:{config['api_port']}",
            "--data-dir", str(root / "data/legacy"),
            "--qpdf-library", str(target / "lib/libqpdf.so.30.4.1"),
            "--ffprobe-path", str(target / "bin/ffprobe"),
            "--ffmpeg-path", str(target / "bin/ffmpeg"),
            "--signer-cert", str(ca / "certs/qadra-server.crt.pem"),
            "--signer-key", str(ca / "private/qadra-server.key.pem"),
            "--ca-cert", str(ca / "ca.crt.pem"), "--crl", str(ca / "crl/crl.pem"),
            "--tsa-config", str(target / "pki/tsa.cnf"), "--tsa-dir", str(root / "data/tsa")]
    os.chdir(root / "data")
    os.execve(str(binary), args, env)


if __name__ == "__main__":
    import sys
    root = Path(sys.argv[1]).resolve(strict=True)
    if sys.argv[2:] == ["--wait-api"]:
        Runtime(root).wait_api()
    else:
        serve(root)
