"""Installer command budgets remain compatible with the real bounded runner."""
import importlib
from pathlib import Path
import time
import unittest

from deploy_controller_publication_support import REPOSITORY


class InstallationCommandTests(unittest.TestCase):
    def test_fresh_reopen_budget_runs_an_innocuous_command_with_the_real_runner(self):
        product = importlib.import_module("controller_installation")
        current = product.Installation.__new__(product.Installation)
        current.deadline = time.monotonic() + 600
        current.target = {"systemctl": {"path": str(Path("/bin/true").resolve(strict=True))}}
        current.env = {}
        self.assertEqual(current.command("start", ("synthetic.service",)), "")

    def test_insufficient_budget_rejects_before_starting_any_command(self):
        product = importlib.import_module("controller_installation")
        current = product.Installation.__new__(product.Installation)
        current.deadline = time.monotonic() + 239
        current.target = {"systemctl": {"path": "/unavailable-synthetic-command"}}
        current.env = {}
        with self.assertRaisesRegex(RuntimeError, "configured start budget"):
            current.command("start", ("synthetic.service",))
