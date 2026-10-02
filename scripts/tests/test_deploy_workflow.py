"""Deployment must use owned runners and retain reusable verification rights."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


class DeploymentWorkflowTests(unittest.TestCase):
    def test_all_executable_jobs_use_owned_runners(self):
        text = (ROOT / ".github/workflows/deploy.yml").read_text()
        runners = re.findall(r"^    runs-on: (.+)$", text, re.MULTILINE)
        self.assertEqual(len(runners), 3)
        self.assertTrue(all("self-hosted" in runner for runner in runners))
        package = text.split("  package:\n", 1)[1].split("  deploy:\n", 1)[0]
        self.assertIn("runs-on: [self-hosted, Linux, X64, tt-ci-dedicated]", package)

    def test_reused_gates_can_cancel_the_failed_release_campaign(self):
        text = (ROOT / ".github/workflows/deploy.yml").read_text()
        permissions = text.split("permissions:\n", 1)[1].split("\n\n", 1)[0]
        self.assertIn("actions: write", permissions)
        for name in ("ci", "web"):
            gate = (ROOT / f".github/workflows/{name}.yml").read_text()
            self.assertIn("  workflow_call:", gate)
        self.assertIn("needs: [version, rust, web]", text)


if __name__ == "__main__":
    unittest.main()
