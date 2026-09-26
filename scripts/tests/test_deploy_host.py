"""Host configuration keeps all listeners private and secrets out of units."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
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

    def test_units_use_private_permissions_and_no_secret_values(self):
        units = service_units(Path("/home/hat/qadra"), "/usr/lib/postgresql/16/bin")
        self.assertEqual(set(units), {"qadra-api", "qadra-web", "qadra-postgres", "qadra-redis"})
        for unit in units.values():
            self.assertIn("UMask=0077", unit)
            self.assertNotIn("PASSWORD", unit)
            self.assertIn("WantedBy=default.target", unit)


if __name__ == "__main__":
    unittest.main()
