"""Unit candidates preserve unrelated bytes and carry one explicit approval."""
import copy
import json
import os
from pathlib import Path
import shlex
import subprocess
import unittest

from deploy_controller_approval_support import (
    Fixture, LAUNCHER, PYTHON, REJECTIONS, approver, expected_units, renderer, tree,
)
from deploy_controller_launcher_support import close_child
from deploy_controller_publication_support import exchange_fixture_directories


class ControllerUnitCommandTests(unittest.TestCase):
    def setUp(self):
        self.product = renderer()
        self.approver = approver()

    def test_four_units_preserve_every_unrelated_byte_and_use_the_literal_approved_hash(self):
        fixture = Fixture(self)
        receipt = fixture.approve(self.approver)
        units = fixture.units()
        original = copy.deepcopy(units)
        before = tree(fixture.root)
        rendered = fixture.render(self.product, receipt, units)
        self.assertEqual(rendered, expected_units(fixture, original, receipt["installed_sha256"]))
        self.assertEqual(units, original)
        self.assertEqual(tree(fixture.root), before)
        for name, value in rendered.items():
            with self.subTest(unit=name):
                self.assertEqual(value.count(receipt["installed_sha256"].encode()), 1)
                self.assertIn(b" -I -B -S ", value)
                self.assertNotIn(str(fixture.tools).encode(), value)

    def test_renderer_rejects_wrong_approval_identity_unsafe_paths_and_ambiguous_directives(self):
        fixture = Fixture(self)
        receipt = fixture.approve(self.approver)
        baseline = fixture.units()
        approval = json.loads(fixture.current_approval.read_bytes())
        attacks = ("receipt_only", "changed_record", "approval_hash", "other_root", "other_inode",
                   "missing_unit", "duplicate_start", "duplicate_prestart", "unknown_command",
                   "empty_override", "python_meta", "launcher_specifier", "launcher_in_tools")
        before = tree(fixture.root)
        for attack in attacks:
            with self.subTest(attack=attack):
                units, overrides = copy.deepcopy(baseline), {}
                if attack == "receipt_only":
                    overrides["approval"] = receipt
                elif attack == "changed_record":
                    overrides["approval"] = {**approval, "installed_sha256": "0" * 64}
                elif attack == "approval_hash":
                    overrides["expected_approval_sha256"] = "0" * 64
                elif attack == "other_root":
                    overrides["root"] = fixture.base / "other-root"
                elif attack == "other_inode":
                    overrides["expected_root_identity"] = {
                        key: approval["root"][key] for key in ("uid", "device", "inode")}
                    overrides["expected_root_identity"]["inode"] += 1
                elif attack == "missing_unit":
                    del units["qadra-redis.service"]
                elif attack in ("duplicate_start", "empty_override"):
                    extra = b"ExecStart=/unreviewed/program\n" if attack == "duplicate_start" else b"ExecStart=\n"
                    units["qadra-api.service"] = units["qadra-api.service"].replace(b"[Install]", extra + b"[Install]")
                elif attack == "duplicate_prestart":
                    units["qadra-web.service"] += b"\n[Service]\nExecStartPre=/unreviewed/program\n"
                elif attack == "unknown_command":
                    units["qadra-api.service"] = units["qadra-api.service"].replace(b"/tools/runtime.py", b"/tools/other.py")
                elif attack == "python_meta":
                    overrides["python_executable"] = Path("/private/python;unexpected")
                elif attack == "launcher_specifier":
                    overrides["launcher_path"] = Path("/private/%h/controller_launcher.py")
                else:
                    overrides["launcher_path"] = fixture.tools / "controller_launcher.py"
                with self.assertRaises(REJECTIONS):
                    fixture.render(self.product, receipt, units, **overrides)
                self.assertEqual(tree(fixture.root), before)

    def test_generated_hash_never_adopts_current_tools_on_a_generation_mismatch(self):
        fixture = Fixture(self)
        first = fixture.approve(self.approver)
        units = fixture.units()
        first_units = fixture.render(self.product, first, units)
        fixture.advance()
        second = fixture.approve(self.approver, first["approval_sha256"])
        second_units = fixture.render(self.product, second, units)
        approval_before = tree(fixture.current_approval)
        environment = {**os.environ, "CONTROLLER_APPROVAL_MARKER": str(fixture.marker)}

        def run(value, expected_generation):
            lines = value.decode("ascii").splitlines()
            command = shlex.split(next(line[len("ExecStart="):] for line in lines if line.startswith("ExecStart=")))
            self.assertEqual(command[:5], [str(PYTHON), "-I", "-B", "-S", str(LAUNCHER)])
            process = subprocess.Popen(command, cwd=fixture.base, env=environment,
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                output, error = process.communicate(timeout=5)
                if expected_generation is None:
                    self.assertNotEqual(process.returncode, 0)
                    self.assertEqual(output, "")
                    self.assertFalse(fixture.marker.exists(), "unapproved controller entered")
                else:
                    self.assertEqual((process.returncode, error), (0, ""))
                    self.assertEqual(json.loads(output), {"generation": expected_generation})
                    self.assertEqual(fixture.marker.read_text(), "entered")
                    fixture.marker.unlink()
            finally:
                close_child(process)
            self.assertEqual(tree(fixture.current_approval), approval_before)
            fixture.assert_preserved(self)

        run(first_units["qadra-api.service"], None)
        run(second_units["qadra-api.service"], "B")
        exchange_fixture_directories(fixture.tools, fixture.exchange)
        run(second_units["qadra-api.service"], None)
        run(first_units["qadra-api.service"], "A")


if __name__ == "__main__":
    unittest.main()
