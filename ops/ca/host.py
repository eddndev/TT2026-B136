"""Render private CA service configuration without installing or changing state.

The caller supplies an already accepted literal release directory and verifies
its manifest before installation. Syntax validation here does not admit a release
or inspect the filesystem. The existing database services and CA are preserved.
"""
from pathlib import Path
import re


def _path(value):
    if not isinstance(value, (str, Path)):
        raise ValueError("CA paths must be explicit absolute paths")
    text = str(value)
    if (not re.fullmatch(r"/[A-Za-z0-9_./-]+", text) or text == "/"
            or any(part in ("", ".", "..") for part in text.split("/")[1:])):
        raise ValueError("CA paths must be absolute without whitespace or metacharacters")
    return Path(text)


def _locations(root, release):
    root, release = _path(root), _path(release)
    if release.parent != root / "releases" or release.name in ("current", "previous"):
        raise ValueError("CA configuration requires a literal accepted release directory")
    return root, release


def nginx_config(root, release, *, web_port=18088, api_port=18089):
    """Render loopback ingress for a separately installed CA web service."""
    root, release = _locations(root, release)
    if (any(type(port) is not int or not 1 <= port <= 65535 for port in (web_port, api_port))
            or web_port == api_port):
        raise ValueError("CA web and API require distinct valid integer ports")
    return f"""worker_processes 1;
pid {root}/run/ca/nginx.pid;
error_log {root}/logs/ca/nginx-error.log;
events {{ worker_connections 128; }}
http {{
    include /etc/nginx/mime.types;
    default_type application/octet-stream;
    access_log {root}/logs/ca/nginx-access.log;
    client_body_temp_path {root}/run/ca/client-body;
    proxy_temp_path {root}/run/ca/proxy;
    fastcgi_temp_path {root}/run/ca/fastcgi;
    uwsgi_temp_path {root}/run/ca/uwsgi;
    scgi_temp_path {root}/run/ca/scgi;
    server_tokens off;
    server {{
        listen 127.0.0.1:{web_port};
        server_name localhost;
        root {release}/ca-web;
        index index.html;
        client_max_body_size 32k;
        add_header X-Content-Type-Options nosniff always;
        add_header Referrer-Policy same-origin always;
        location /api/ {{
            proxy_pass http://127.0.0.1:{api_port};
            proxy_http_version 1.1;
            proxy_set_header Host $host;
            proxy_set_header X-Forwarded-For $remote_addr;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_read_timeout 180s;
            proxy_send_timeout 180s;
            proxy_cache off;
            proxy_hide_header Cache-Control;
            add_header Cache-Control no-store always;
            add_header X-Content-Type-Options nosniff always;
            add_header Referrer-Policy same-origin always;
        }}
        location = /version.json {{ add_header Cache-Control no-store always; }}
        location / {{ try_files $uri $uri/ =404; }}
    }}
}}
"""


def service_units(root, release, python_bin, nginx_bin="/usr/sbin/nginx"):
    """Render only CA user units; never depend on the stopped office ingress."""
    root, release = _locations(root, release)
    python_bin, nginx_bin = _path(python_bin), _path(nginx_bin)
    commands = {
        "qadra-ca": f"{release}/bin/qadra-ca --config {root}/config/ca-service.json",
        "qadra-ca-web": f"{nginx_bin} -c {root}/config/ca-nginx.conf -g 'daemon off;'",
    }
    units = {}
    for name, command in commands.items():
        dependencies = ("qadra-postgres.service qadra-redis.service"
                        if name == "qadra-ca" else "qadra-ca.service")
        memory = "2G" if name == "qadra-ca" else "128M"
        # Direct script execution retains its own imports; -I would remove them.
        prestart = f"{python_bin} -B {release}/ops/deploy/restore_fence.py {root}"
        units[name] = f"""[Unit]
Description=Qadra private {name}
Requires={dependencies}
After={dependencies}

[Service]
Type=simple
WorkingDirectory={root}/data
ExecStartPre={prestart}
ExecStart={command}
MemoryMax={memory}
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
