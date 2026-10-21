"""Deployment must use owned runners and retain reusable verification rights."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


class DeploymentWorkflowTests(unittest.TestCase):
    def test_all_executable_jobs_use_owned_runners(self):
        text = (ROOT / ".github/workflows/deploy.yml").read_text()
        runners = re.findall(r"^    runs-on: (.+)$", text, re.MULTILINE)
        self.assertEqual(len(runners), 4)
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
        self.assertIn("needs: [version, gates, rust, web]", text)

    def test_gates_are_skipped_only_when_reuse_was_proven(self):
        text = (ROOT / ".github/workflows/deploy.yml").read_text()
        gates = text.split("  gates:\n", 1)[1].split("  rust:\n", 1)[0]
        self.assertIn("python3 -B scripts/deploy_gate_reuse.py", gates)
        self.assertIn("persist-credentials: false", gates)
        package = text.split("  package:\n", 1)[1].split("  deploy:\n", 1)[0]
        condition = package.split("    runs-on:", 1)[0]
        for clause in ("!cancelled()", "needs.version.result == 'success'",
                       "needs.gates.result == 'success'",
                       "(needs.rust.result == 'success' || (needs.rust.result == 'skipped' && needs.gates.outputs.reuse_rust == 'true'))",
                       "(needs.web.result == 'success' || (needs.web.result == 'skipped' && needs.gates.outputs.reuse_web == 'true'))"):
            self.assertIn(clause, condition)
        self.assertIn("run-id: ${{ needs.gates.outputs.web_run_id }}", package)

    def test_manual_preparation_builds_exact_commit_without_deployment(self):
        text = (ROOT / ".github/workflows/deploy.yml").read_text()
        triggers = text.split("\npermissions:\n", 1)[0]
        self.assertIn("  workflow_dispatch:\n", triggers)
        manual = triggers.split("  workflow_dispatch:\n", 1)[1]
        option = re.search(
            r"(?m)^      prepare_version:\n((?:        [^\n]*\n)+)", manual
        )
        self.assertIsNotNone(option, "manual preparation needs a version input")
        self.assertIn("required: true", option.group(1))
        self.assertIn("type: string", option.group(1))

        version = text.split("  version:\n", 1)[1].split("  rust:\n", 1)[0]
        self.assertIn("github.event_name == 'workflow_dispatch'", version)
        self.assertIn("inputs.prepare_version", version)
        self.assertIn('validate_version(os.environ["TAG"])', version)
        self.assertRegex(
            version,
            r'commit=\$\(git rev-parse (?:HEAD|"\$GITHUB_SHA\^\{commit\}")\)',
        )
        self.assertIn("commit: ${{ steps.identity.outputs.commit }}", version)
        for name, following, workflow in (
            ("rust", "web", "ci"),
            ("web", "package", "web"),
        ):
            gate = text.split(f"  {name}:\n", 1)[1].split(
                f"  {following}:\n", 1
            )[0]
            self.assertIn("needs: [version, gates]", gate)
            self.assertIn(f"uses: ./.github/workflows/{workflow}.yml", gate)
            reuse = "reuse_rust" if name == "rust" else "reuse_web"
            self.assertRegex(gate, rf"(?m)^    if: \$\{{\{{ needs\.gates\.outputs\.{reuse} != 'true' \}}\}}$")

        package, deploy = text.split("  package:\n", 1)[1].split("  deploy:\n", 1)
        self.assertIn("needs: [version, gates, rust, web]", package)
        self.assertIn("ref: ${{ needs.version.outputs.commit }}", package)
        self.assertIn("SOURCE_COMMIT: ${{ needs.version.outputs.commit }}", package)
        self.assertIn('bash scripts/build-release.sh "$VERSION" "$SOURCE_COMMIT"', package)
        self.assertIn("uses: actions/upload-artifact@v4", package)
        condition = re.search(r"(?m)^    if: (.+)$", deploy)
        self.assertIsNotNone(condition, "manual preparation must never activate")
        self.assertIn("github.event_name == 'push'", condition.group(1))
        self.assertIn("startsWith(github.ref, 'refs/tags/')", condition.group(1))
        self.assertNotIn("||", condition.group(1))


if __name__ == "__main__":
    unittest.main()
