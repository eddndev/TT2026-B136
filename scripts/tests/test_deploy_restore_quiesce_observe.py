"""Read cgroup and proc fixtures without systemd, signals or real sockets."""
import importlib
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))

GROUP = "/user.slice/qadra-api.service"
HEADER = "  sl  local_address rem_address st tx_queue rx_queue tr tm->when retrnsmt uid timeout inode\n"


class QuiesceObservationTests(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name).resolve()
        self.proc, self.cgroups = self.root / "proc", self.root / "cgroup"
        self.proc.mkdir()
        self.cgroups.mkdir()
        (self.cgroups / "cgroup.controllers").write_text("cpu memory pids\n")
        self.group = self.cgroups / GROUP.lstrip("/")
        self.group.mkdir(parents=True)
        (self.group / "cgroup.procs").write_text("")
        (self.proc / "net").mkdir()
        for name in ("tcp", "tcp6"):
            (self.proc / "net" / name).write_text(HEADER)

    def module(self):
        return importlib.import_module("restore_quiesce_observe")

    def process(self, pid=4101, ticks=12345, group=GROUP, uid=None):
        uid = os.getuid() if uid is None else uid
        directory = self.proc / str(pid)
        directory.mkdir()
        tail = ["S"] + ["0"] * 18 + [str(ticks)] + ["0"] * 20
        (directory / "stat").write_text(f"{pid} (worker ) name) " + " ".join(tail) + "\n")
        (directory / "status").write_text(f"Name:\tworker\nUid:\t{uid}\t{uid}\t{uid}\t{uid}\n")
        (directory / "cgroup").write_text(f"0::{group}\n")
        target = self.cgroups / group.lstrip("/")
        target.mkdir(parents=True, exist_ok=True)
        with (target / "cgroup.procs").open("a") as stream:
            stream.write(str(pid) + "\n")
        return directory

    def snapshot(self, module, group=GROUP, timeout=5):
        return module.process_snapshot(group, timeout=timeout, proc_root=self.proc, cgroup_root=self.cgroups)

    def sockets(self, module, ports=(18086, 18087, 15486, 16386), timeout=5):
        return module.listening_ports(ports, timeout=timeout, proc_root=self.proc)

    def row(self, slot, address, port, state="0A"):
        return (f"{slot}: {address}:{port:04X} {'0' * len(address)}:0000 {state} "
                "00000000:00000000 00:00000000 00000000 1000 0 12345\n")

    def test_process_snapshot_preserves_pid_start_time_uid_and_child_cgroup(self):
        self.process()
        self.process(4102, 54321, GROUP + "/worker")
        module = self.module()
        with patch.object(os, "kill", side_effect=AssertionError("observation must not signal")):
            result = self.snapshot(module)
        self.assertEqual(sorted(result, key=lambda row: row["pid"]), [
            {"pid": 4101, "uid": os.getuid(), "start_ticks": 12345, "control_group": GROUP},
            {"pid": 4102, "uid": os.getuid(), "start_ticks": 54321, "control_group": GROUP + "/worker"},
        ])

    def test_missing_group_is_empty_only_with_valid_v2_root_and_safe_group_path(self):
        module = self.module()
        self.assertEqual(self.snapshot(module, GROUP + "/absent"), [])
        for group in ("relative", "/../proc", GROUP + "/../foreign", "/", GROUP + "//child"):
            with self.subTest(group=group), self.assertRaises((ValueError, RuntimeError)):
                self.snapshot(module, group)
        (self.cgroups / "cgroup.controllers").unlink()
        with self.assertRaises((ValueError, RuntimeError, OSError)):
            self.snapshot(module, GROUP + "/absent")

    def test_ambiguous_membership_or_process_metadata_never_claims_an_empty_group(self):
        directory = self.process()
        originals = {name: (directory / name).read_text() for name in ("stat", "status", "cgroup")}
        module = self.module()
        variants = (
            ("stat", originals["stat"].replace("4101", "9999", 1)),
            ("stat", "4101 (broken) S 0\n"),
            ("status", "Name: worker\n"),
            ("status", originals["status"] + "Uid: 1 1 1 1\n"),
            ("status", "Uid: 1 2 1 1\n"),
            ("cgroup", "0::/foreign.service\n"),
            ("cgroup", originals["cgroup"] * 2),
        )
        for name, content in variants:
            with self.subTest(field=name, content=content):
                (directory / name).write_text(content)
                with self.assertRaises((ValueError, RuntimeError)):
                    self.snapshot(module)
                (directory / name).write_text(originals[name])
        for content in ("4101\n4101\n", "not-a-pid\n", "0\n", "-1\n"):
            (self.group / "cgroup.procs").write_text(content)
            with self.subTest(membership=content), self.assertRaises((ValueError, RuntimeError)):
                self.snapshot(module)

    def test_redirected_cgroup_or_proc_files_and_missing_process_reject(self):
        directory = self.process()
        module = self.module()
        status = directory / "status"
        content = status.read_bytes()
        outside = self.root / "outside-status"
        outside.write_bytes(content)
        status.unlink()
        status.symlink_to(outside)
        with self.assertRaises((ValueError, RuntimeError, OSError)):
            self.snapshot(module)
        status.unlink()
        with self.assertRaises((ValueError, RuntimeError, OSError)):
            self.snapshot(module)
        status.write_bytes(content)
        (self.group / "foreign").symlink_to(self.root, target_is_directory=True)
        with self.assertRaises((ValueError, RuntimeError, OSError)):
            self.snapshot(module)

    def test_pid_reuse_during_observation_rejects_changed_start_time(self):
        directory = self.process()
        module = self.module()
        original_read = module.read
        reads = 0
        def changed(path, maximum, deadline):
            nonlocal reads
            if Path(path) == directory / "stat":
                reads += 1
                if reads == 2:
                    path.write_text(path.read_text().replace("12345", "54321"))
            return original_read(path, maximum, deadline)
        with patch.object(module, "read", side_effect=changed):
            with self.assertRaises((ValueError, RuntimeError)):
                self.snapshot(module)

    def test_ipv4_ipv6_wildcard_and_loopback_listeners_block_only_requested_ports(self):
        (self.proc / "net/tcp").write_text(HEADER + self.row(0, "0100007F", 18086)
            + self.row(1, "00000000", 15486) + self.row(2, "0100007F", 18087, "01"))
        (self.proc / "net/tcp6").write_text(HEADER + self.row(0, "0" * 32, 16386)
            + self.row(1, "0" * 31 + "1", 19999))
        self.assertEqual(self.sockets(self.module()), [15486, 16386, 18086])

    def test_malformed_missing_or_redirected_network_table_is_not_no_listeners(self):
        module = self.module()
        tcp = self.proc / "net/tcp"
        valid = HEADER + self.row(0, "0100007F", 18086)
        for content in ("", "wrong header\n", HEADER + "short row\n",
                        valid.replace(":46A6", ":ZZZZ"), valid.replace(" 0A ", " QQ ")):
            tcp.write_text(content)
            with self.subTest(table=content), self.assertRaises((ValueError, RuntimeError)):
                self.sockets(module)
        tcp.write_text(valid)
        other = self.proc / "net/tcp6"
        other.unlink()
        with self.assertRaises((ValueError, RuntimeError, OSError)):
            self.sockets(module)
        other.symlink_to(tcp)
        with self.assertRaises((ValueError, RuntimeError, OSError)):
            self.sockets(module)

    def test_invalid_ports_timeouts_large_inputs_and_expired_budget_reject(self):
        module = self.module()
        for ports in ((), (True,), (0,), (65536,), (18086, 18086), tuple(range(1, 6))):
            with self.subTest(ports=ports), self.assertRaises((ValueError, RuntimeError)):
                self.sockets(module, ports)
        for timeout in (0, -1, True, float("inf")):
            with self.subTest(timeout=timeout), self.assertRaises((ValueError, RuntimeError)):
                self.snapshot(module, timeout=timeout)
        (self.proc / "net/tcp").write_bytes(b"x" * (1024 * 1024 + 1))
        with self.assertRaises((ValueError, RuntimeError)):
            self.sockets(module)
        with patch.object(module, "monotonic", side_effect=[0, 10, 10, 10, 10]):
            with self.assertRaises((ValueError, RuntimeError, TimeoutError)):
                self.snapshot(module, timeout=1)
