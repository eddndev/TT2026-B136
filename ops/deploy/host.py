"""Prepare a private, persistent deployment owned by the current SSH user."""
import argparse
import base64
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import socket
import subprocess
import sys


def validate_root(value):
    if (not re.fullmatch(r"/[A-Za-z0-9_./-]+", value) or value == "/"
            or ".." in value.split("/") or "//" in value):
        raise ValueError("root must be an absolute path without whitespace or metacharacters")
    return Path(value)


def nginx_config(root, config):
    return f"""worker_processes 1;
pid {root}/run/nginx.pid;
error_log {root}/logs/nginx-error.log;
events {{ worker_connections 128; }}
http {{
    include /etc/nginx/mime.types;
    default_type application/octet-stream;
    access_log {root}/logs/nginx-access.log;
    client_body_temp_path {root}/run/client-body;
    proxy_temp_path {root}/run/proxy;
    fastcgi_temp_path {root}/run/fastcgi;
    uwsgi_temp_path {root}/run/uwsgi;
    scgi_temp_path {root}/run/scgi;
    server_tokens off;
    server {{
        listen 127.0.0.1:{config['web_port']};
        server_name localhost;
        root {root}/current/web;
        index index.html;
        client_max_body_size 18m;
        add_header X-Content-Type-Options nosniff always;
        add_header Referrer-Policy same-origin always;
        location = /api/v1/auth/bootstrap {{ return 403; }}
        location /api/ {{
            proxy_pass http://127.0.0.1:{config['api_port']};
            proxy_http_version 1.1;
            proxy_set_header Host $host;
            proxy_set_header X-Forwarded-For $remote_addr;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_read_timeout 180s;
        }}
        location = /version.json {{ add_header Cache-Control no-store; }}
        location / {{ try_files $uri $uri/ =404; }}
    }}
}}
"""


def service_units(root, postgres_bin, python_bin=None, redis_bin="/usr/bin/redis-server"):
    python_bin = python_bin or sys.executable
    commands = {
        "qadra-postgres": f"{postgres_bin}/postgres -D {root}/data/postgres",
        "qadra-redis": f"{redis_bin} {root}/config/redis.conf",
        "qadra-api": f"{python_bin} {root}/tools/runtime.py {root}",
        "qadra-web": f"/usr/sbin/nginx -c {root}/config/nginx.conf -g 'daemon off;'",
    }
    units = {}
    for name, command in commands.items():
        dependencies = ""
        if name == "qadra-api":
            dependencies = "Requires=qadra-postgres.service qadra-redis.service\nAfter=qadra-postgres.service qadra-redis.service\n"
        elif name == "qadra-web":
            dependencies = "Requires=qadra-api.service\nAfter=qadra-api.service\n"
        prestart = (f"ExecStartPre={python_bin} {root}/tools/runtime.py {root} --wait-api\n"
                    if name == "qadra-web" else "")
        if name in ("qadra-postgres", "qadra-redis"):
            prestart = f"ExecStartPre={python_bin} {root}/tools/restore_fence.py {root}\n"
        memory = {"qadra-api": "2G", "qadra-postgres": "768M", "qadra-redis": "256M", "qadra-web": "128M"}[name]
        units[name] = f"""[Unit]
Description=Qadra private {name}
{dependencies}
[Service]
Type=simple
WorkingDirectory={root}/data
ExecStart={command}
{prestart}MemoryMax={memory}
CPUQuota=200%
Restart=on-failure
RestartSec=5
TimeoutStopSec=90
TimeoutStartSec=240
KillSignal=SIGINT
UMask=0077
NoNewPrivileges=yes
PrivateTmp=yes

[Install]
WantedBy=default.target
"""
    return units


def prepare(root):
    from restore_fence import guard
    guard(root)
    os.umask(0o077)
    for executable in ("psql", "pg_dump", "pg_restore", "pg_config", "redis-server",
                       "redis-cli", "redis-check-rdb", "nginx", "openssl", "python3", "systemctl"):
        if shutil.which(executable) is None:
            raise ValueError(f"missing host dependency: {executable}")
    postgres_bin = subprocess.check_output(["pg_config", "--bindir"], text=True).strip()
    for name in ("initdb", "postgres"):
        if not (Path(postgres_bin) / name).is_file():
            raise ValueError(f"missing PostgreSQL server tool: {name}")
    for name in ("config", "data/tmp", "data/legacy", "backups", "releases", "incoming",
                 "tools", "logs", "run/client-body", "run/proxy", "run/fastcgi",
                 "run/uwsgi", "run/scgi", "run/pg"):
        (root / name).mkdir(parents=True, exist_ok=True, mode=0o700)
    (root / "data/.env").touch()
    config_file = root / "config/settings.json"
    if not config_file.exists():
        config = {"web_port": 18086, "api_port": 18087, "postgres_port": 15486, "redis_port": 16386,
                  "admin_password": secrets.token_hex(32), "runtime_password": secrets.token_hex(32),
                  "redis_password": secrets.token_hex(32), "kek": base64.b64encode(secrets.token_bytes(32)).decode()}
        for port in (config[k] for k in config if k.endswith("_port")):
            with socket.socket() as probe:
                probe.bind(("127.0.0.1", port))
        config_file.write_text(json.dumps(config, indent=2) + "\n")
    config = json.loads(config_file.read_text())
    config_file.chmod(0o600)
    (root / "config/nginx.conf").write_text(nginx_config(root, config))
    (root / "config/redis.conf").write_text(
        f"bind 127.0.0.1\nport {config['redis_port']}\nprotected-mode yes\n"
        f"requirepass {config['redis_password']}\nappendonly yes\n"
        f"dir {root}/data\nappenddirname redis-appendonly\ndbfilename redis.rdb\n")
    for source in Path(__file__).parent.glob("*.py"):
        destination = root / "tools" / source.name
        if source.resolve() != destination.resolve():
            shutil.copyfile(source, destination)
    units = Path.home() / ".config/systemd/user"
    units.mkdir(parents=True, exist_ok=True)
    for name, content in service_units(
            root, postgres_bin, sys.executable, shutil.which("redis-server")).items():
        (units / f"{name}.service").write_text(content)
    subprocess.run(["systemctl", "--user", "daemon-reload"], check=True)
    print(f"Prepared private deployment in {root}; no application was started")


def start_databases(root):
    from restore_fence import guard
    guard(root)
    from provision import create_databases
    create_databases(root)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True)
    parser.add_argument("--start-databases", action="store_true")
    args = parser.parse_args()
    root = validate_root(args.root)
    prepare(root)
    if args.start_databases:
        start_databases(root)
