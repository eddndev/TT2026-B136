"""CA configuration stays pinned, local and independent of office maintenance."""
import importlib.util
from pathlib import Path
import tempfile
import unittest


SOURCE = Path(__file__).resolve().parents[2] / "ops/ca/host.py"
SPEC = importlib.util.spec_from_file_location("ca_host_configuration", SOURCE)
host = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(host)

ROOT = Path("/home/qadra/qadra")
RELEASE = ROOT / "releases/v0.2.0-ca"
PYTHON = "/usr/local/bin/python3.12"
NGINX = "/usr/sbin/nginx"


class CaHostTests(unittest.TestCase):
    def units(self, root=ROOT, release=RELEASE, python=PYTHON, nginx=NGINX):
        return host.service_units(root, release, python, nginx)

    def test_only_ca_api_depends_on_the_existing_databases(self):
        units = self.units()
        self.assertEqual(set(units), {"qadra-ca", "qadra-ca-web"})
        for directive in ("Requires", "After"):
            self.assertIn(f"{directive}=qadra-postgres.service qadra-redis.service\n",
                          units["qadra-ca"])
            self.assertIn(f"{directive}=qadra-ca.service\n", units["qadra-ca-web"])
        for unit in units.values():
            self.assertNotIn("qadra-api.service", unit)
            self.assertNotIn("qadra-web.service", unit)
            self.assertNotIn("PartOf=", unit)
            self.assertIn("UMask=0077", unit)
            self.assertIn("NoNewPrivileges=yes", unit)
            self.assertIn("WantedBy=default.target", unit)

    def test_api_executes_the_literal_release_and_private_config_without_secrets(self):
        units = self.units()
        self.assertIn(f"ExecStart={RELEASE}/bin/qadra-ca --config {ROOT}/config/ca-service.json\n",
                      units["qadra-ca"])
        self.assertIn(f"ExecStart={NGINX} -c {ROOT}/config/ca-nginx.conf -g 'daemon off;'\n",
                      units["qadra-ca-web"])
        for unit in units.values():
            self.assertNotIn("/current/", unit)
            self.assertNotIn("/previous/", unit)
            self.assertNotIn("Environment=", unit)
            self.assertNotIn("EnvironmentFile=", unit)
            self.assertNotIn("enroll-operator", unit)
            self.assertTrue(unit.isascii())

    def test_both_services_guard_restore_using_the_same_pinned_release(self):
        expected = f"ExecStartPre={PYTHON} -B {RELEASE}/ops/deploy/restore_fence.py {ROOT}\n"
        for unit in self.units().values():
            self.assertIn(expected, unit)
            self.assertNotIn(" -I ", unit)
            self.assertNotIn("/tools/", unit)
            self.assertNotIn("crl-maintenance", unit)
            self.assertNotIn("--wait-api", unit)

    def test_nginx_has_local_ca_ports_exact_static_release_and_uncached_bounded_api(self):
        config = host.nginx_config(ROOT, RELEASE)
        self.assertIn("listen 127.0.0.1:18088;", config)
        self.assertIn(f"root {RELEASE}/ca-web;", config)
        self.assertIn("client_max_body_size 32k;", config)
        api = config.split("location /api/ {", 1)[1].split("}", 1)[0]
        self.assertIn("proxy_pass http://127.0.0.1:18089;", api)
        self.assertNotIn("proxy_pass http://127.0.0.1:18089/;", api)
        self.assertIn("proxy_read_timeout 180s;", api)
        self.assertIn("proxy_send_timeout 180s;", api)
        self.assertIn("proxy_hide_header Cache-Control;", api)
        self.assertIn("add_header Cache-Control no-store always;", api)
        self.assertIn("proxy_cache off;", api)
        self.assertNotIn("listen 0.0.0.0", config)
        self.assertNotIn("listen [::]", config)
        self.assertNotIn("/current/", config)
        self.assertTrue(config.isascii())

    def test_nginx_pid_logs_and_temporary_paths_are_separate_from_the_office(self):
        config = host.nginx_config(ROOT, RELEASE)
        self.assertIn(f"pid {ROOT}/run/ca/nginx.pid;", config)
        self.assertIn(f"error_log {ROOT}/logs/ca/nginx-error.log;", config)
        self.assertIn(f"access_log {ROOT}/logs/ca/nginx-access.log;", config)
        for directive, name in (("client_body", "client-body"), ("proxy", "proxy"),
                                ("fastcgi", "fastcgi"), ("uwsgi", "uwsgi"),
                                ("scgi", "scgi")):
            self.assertIn(f"{directive}_temp_path {ROOT}/run/ca/{name};", config)
        self.assertNotIn(f"pid {ROOT}/run/nginx.pid;", config)

    def test_rendering_does_not_create_or_install_anything(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "not-installed"
            release = root / "releases/accepted"
            host.nginx_config(root, release, web_port=28088, api_port=28089)
            units = self.units(root, release)
            self.assertFalse(root.exists())
            self.assertNotIn("qadra-postgres", units)
            self.assertNotIn("qadra-redis", units)

    def test_paths_cannot_inject_directives_or_select_a_moving_release(self):
        invalid = ("relative", "/", "/tmp/a b", "/tmp/x%h", "/tmp/$HOME", "/tmp/a;bad",
                   "/tmp/../other", "/tmp//other", "/tmp/line\nExecStart=bad", "/tmp/ca\u00e9")
        for value in invalid:
            with self.subTest(root=value), self.assertRaises(ValueError):
                self.units(root=value)
            with self.subTest(nginx_root=value), self.assertRaises(ValueError):
                host.nginx_config(value, RELEASE)
            with self.subTest(python=value), self.assertRaises(ValueError):
                self.units(python=value)
            with self.subTest(nginx=value), self.assertRaises(ValueError):
                self.units(nginx=value)
        for release in (*invalid, ROOT / "current", ROOT / "previous",
                        ROOT / "releases", ROOT / "releases/name/nested", "/other/releases/name"):
            with self.subTest(release=release), self.assertRaises(ValueError):
                self.units(release=release)
            with self.subTest(nginx_release=release), self.assertRaises(ValueError):
                host.nginx_config(ROOT, release)

    def test_ports_are_integers_in_range_and_cannot_share_a_listener(self):
        for value in (None, True, False, 0, -1, 65536, "18088", "18088; bad", 18088.0):
            with self.subTest(web_port=value), self.assertRaises(ValueError):
                host.nginx_config(ROOT, RELEASE, web_port=value)
            with self.subTest(api_port=value), self.assertRaises(ValueError):
                host.nginx_config(ROOT, RELEASE, api_port=value)
        with self.assertRaises(ValueError):
            host.nginx_config(ROOT, RELEASE, web_port=18089, api_port=18089)
        config = host.nginx_config(ROOT, RELEASE, web_port=28088, api_port=28089)
        self.assertIn("listen 127.0.0.1:28088;", config)
        self.assertIn("proxy_pass http://127.0.0.1:28089;", config)


if __name__ == "__main__":
    unittest.main()
