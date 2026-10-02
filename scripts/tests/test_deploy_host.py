"""Host configuration keeps all listeners private and secrets out of units."""
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
import host
from host import nginx_config, service_units, validate_root


class HostTests(unittest.TestCase):
    def test_rejects_paths_unsafe_in_service_configuration(self):
        for value in ("relative", "/tmp/a b", "/tmp/x%h", "/tmp/a;bad", "/tmp/../etc", "/"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                validate_root(value)

    def test_nginx_binds_loopback_and_proxies_api_without_rewriting(self):
        config = nginx_config(Path("/home/hat/qadra"), {"web_port": 18086, "api_port": 18087})
        self.assertIn("listen 127.0.0.1:18086", config)
        self.assertIn("proxy_pass http://127.0.0.1:18087;", config)
        self.assertIn("location = /api/v1/auth/bootstrap", config)
        self.assertIn("return 403", config)
        self.assertIn("/current/web", config)

    def test_prepare_keeps_every_nginx_temporary_directory_private(self):
        with tempfile.TemporaryDirectory() as directory:
            temporary = Path(directory)
            root = temporary / "qadra"
            postgres = temporary / "pg-bin"
            postgres.mkdir()
            for name in ("initdb", "postgres"):
                (postgres / name).touch()
            with patch.object(host.os, "umask"), \
                    patch.object(host.shutil, "which", side_effect=lambda name: "/usr/bin/" + name), \
                    patch.object(host.subprocess, "check_output", return_value=str(postgres)), \
                    patch.object(host.subprocess, "run"), \
                    patch.object(host.socket, "socket"), \
                    patch.object(host.Path, "home", return_value=temporary / "home"):
                host.prepare(root)
            config = (root / "config/nginx.conf").read_text()
            for directive, name in (("client_body", "client-body"), ("proxy", "proxy"),
                                    ("fastcgi", "fastcgi"), ("uwsgi", "uwsgi"),
                                    ("scgi", "scgi")):
                with self.subTest(protocol=name):
                    path = root / "run" / name
                    self.assertIn(f"{directive}_temp_path {path};", config)
                    self.assertTrue(path.is_dir())
                    self.assertEqual(path.stat().st_mode & 0o777, 0o700)

    def test_units_use_private_permissions_and_no_secret_values(self):
        units = service_units(Path("/home/hat/qadra"), "/usr/lib/postgresql/16/bin")
        self.assertEqual(set(units), {"qadra-api", "qadra-web", "qadra-postgres", "qadra-redis"})
        for unit in units.values():
            self.assertIn("UMask=0077", unit)
            self.assertNotIn("PASSWORD", unit)
            self.assertIn("WantedBy=default.target", unit)

    def test_units_use_the_verified_host_interpreter_and_redis_binary(self):
        units = service_units(Path("/home/qadra/qadra"), "/usr/lib/postgresql/16/bin",
                              "/usr/local/bin/python3", "/usr/local/bin/redis-server")
        self.assertIn("ExecStart=/usr/local/bin/python3 ", units["qadra-api"])
        self.assertIn("ExecStartPre=/usr/local/bin/python3 ", units["qadra-web"])
        self.assertIn("ExecStart=/usr/local/bin/redis-server ", units["qadra-redis"])


if __name__ == "__main__":
    unittest.main()
