"""A main push reuses tests only from a successful pull request run of the same tree."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ci_tree_reuse as reuse


ROOT = Path(__file__).resolve().parents[2]
TREE = "e" * 40
PARENT = "b" * 40
REPOSITORY_ID = 42


class FakeGitHub:
    def __init__(self, artifacts, runs):
        self.artifacts = artifacts
        self.runs = runs
        self.requests = []

    def get(self, path, **query):
        self.requests.append((path, query))
        if path == "actions/artifacts":
            return {"artifacts": self.artifacts}
        return self.runs[int(path.rsplit("/", 1)[1])]


def artifact(run_id, *, expired=False, name=None):
    return {"name": name or f"tested-tree-{TREE}", "expired": expired,
            "workflow_run": {"id": run_id, "repository_id": REPOSITORY_ID,
                             "head_repository_id": REPOSITORY_ID}}


def run(run_id, **overrides):
    value = {"id": run_id, "path": ".github/workflows/ci.yml", "event": "pull_request",
             "status": "completed", "conclusion": "success",
             "repository": {"id": REPOSITORY_ID}, "head_repository": {"id": REPOSITORY_ID}}
    value.update(overrides)
    return value


class TreeReuseTests(unittest.TestCase):
    def find(self, artifacts, runs):
        github = FakeGitHub(artifacts, {item["id"]: item for item in runs})
        return reuse.find(TREE, github), github

    def test_successful_pull_request_run_of_the_same_tree_is_reused(self):
        found, github = self.find([artifact(7)], [run(7)])
        self.assertEqual(found, 7)
        self.assertEqual(github.requests[0], ("actions/artifacts", {"name": f"tested-tree-{TREE}", "per_page": 30}))

    def test_runs_that_did_not_fully_pass_are_not_evidence(self):
        for overrides in ({"conclusion": "failure"}, {"conclusion": "cancelled"},
                          {"status": "in_progress", "conclusion": None}):
            with self.subTest(overrides=overrides):
                found, _ = self.find([artifact(7)], [run(7, **overrides)])
                self.assertIsNone(found)

    def test_other_workflows_events_and_forks_are_ignored(self):
        for overrides in ({"path": ".github/workflows/web.yml"}, {"event": "push"},
                          {"head_repository": {"id": 99}}):
            with self.subTest(overrides=overrides):
                found, _ = self.find([artifact(7)], [run(7, **overrides)])
                self.assertIsNone(found)

    def test_expired_or_differently_named_artifacts_are_ignored(self):
        found, github = self.find([artifact(7, expired=True), artifact(8, name="tested-tree-" + "f" * 40)],
                                  [run(7), run(8)])
        self.assertIsNone(found)
        self.assertEqual(len(github.requests), 1)

    def test_web_reuse_reads_only_web_pull_request_evidence(self):
        name = f"tested-web-tree-{TREE}"
        web_run = run(7, path=".github/workflows/web.yml")
        github = FakeGitHub([artifact(7, name=name)], {7: web_run})
        self.assertEqual(reuse.find(TREE, github, kind="web"), 7)
        self.assertEqual(github.requests[0], ("actions/artifacts", {"name": name, "per_page": 30}))
        rust_run = FakeGitHub([artifact(7, name=name)], {7: run(7)})
        self.assertIsNone(reuse.find(TREE, rust_run, kind="web"))
        # Rust evidence never stands in for the browser suites, and vice versa.
        self.assertIsNone(reuse.find(TREE, FakeGitHub([artifact(7)], {7: web_run}), kind="web"))

    def test_unknown_kind_is_rejected(self):
        with self.assertRaises(ValueError):
            reuse.find(TREE, FakeGitHub([], {}), kind="docs")

    def parent_runs(self, *runs):
        github = FakeGitHub([], {})
        github.get = lambda path, **query: (github.requests.append((path, query))
                                            or {"workflow_runs": list(runs)})
        return github

    def main_run(self, run_id, **overrides):
        value = {"id": run_id, "path": ".github/workflows/ci.yml", "event": "push",
                 "head_branch": "main", "head_sha": PARENT, "status": "completed",
                 "conclusion": "success"}
        value.update(overrides)
        return value

    def test_web_only_merge_reuses_the_passed_main_run_of_its_parent(self):
        github = self.parent_runs(self.main_run(9))
        self.assertEqual(reuse.equivalent_parent(PARENT, ["web/apps/qadra/x.svelte"], github), 9)
        self.assertEqual(github.requests[0], ("actions/workflows/ci.yml/runs", {
            "branch": "main", "event": "push", "head_sha": PARENT, "status": "success", "per_page": 30}))

    def test_parent_reuse_requires_only_web_changes_and_a_passed_main_push(self):
        for changed in (["web/x", "crates/web/src/lib.rs"], ["Cargo.lock"], ["webx/a"], []):
            with self.subTest(changed=changed):
                self.assertIsNone(reuse.equivalent_parent(PARENT, changed, self.parent_runs(self.main_run(9))))
        for overrides in ({"conclusion": "failure"}, {"event": "pull_request"}, {"head_branch": "x"},
                          {"head_sha": "c" * 40}, {"path": ".github/workflows/web.yml"},
                          {"status": "in_progress"}):
            with self.subTest(overrides=overrides):
                github = self.parent_runs(self.main_run(9, **overrides))
                self.assertIsNone(reuse.equivalent_parent(PARENT, ["web/x"], github))
        with self.assertRaises(ValueError):
            reuse.equivalent_parent("B" * 40, ["web/x"], self.parent_runs())

    def test_tree_must_be_a_full_object_name(self):
        for tree in ("E" * 40, "e" * 39, "", "e" * 40 + "\n"):
            with self.subTest(tree=tree), self.assertRaises(ValueError):
                reuse.find(tree, FakeGitHub([], {}))

    def test_outputs_leave_the_run_empty_without_evidence(self):
        self.assertEqual(reuse.output_lines(7), ["run_id=7"])
        self.assertEqual(reuse.output_lines(None), ["run_id="])


class WebWorkflowTests(unittest.TestCase):
    text = (ROOT / ".github/workflows/web.yml").read_text()

    def job(self, name):
        return self.text.split(f"\n  {name}:\n", 1)[1].split("\n\n  ", 1)[0]

    def test_main_pushes_skip_browser_suites_only_with_web_evidence(self):
        self.assertIn("github.event_name == 'push' && github.ref == 'refs/heads/main'", self.job("reuse"))
        self.assertIn("--kind web", self.job("reuse"))
        for name in ("verify-shards", "integration-shards"):
            self.assertIn("needs.reuse.outputs.run_id == ''", self.job(name))
        self.assertNotIn("needs:", self.job("browser-crypto").split("steps:", 1)[0])

    def test_pull_requests_record_the_tree_after_every_browser_suite(self):
        # The skipped reuse ancestor would skip this job without a status function.
        record = self.job("record-tree")
        self.assertIn("needs: [verify, integration]", record)
        self.assertIn("${{ !cancelled() && github.event_name == 'pull_request' && needs.verify.result == 'success' && needs.integration.result == 'success' }}", record)
        self.assertIn("name: tested-web-tree-${{ steps.tree.outputs.tree }}", record)


class WorkflowTests(unittest.TestCase):
    text = (ROOT / ".github/workflows/ci.yml").read_text()

    def job(self, name, following):
        return self.text.split(f"  {name}:\n", 1)[1].split(f"\n  {following}:\n", 1)[0]

    def test_reuse_runs_only_for_main_pushes_and_tests_run_without_evidence(self):
        reuse_job = self.job("reuse", "test")
        self.assertIn("fetch-depth: 2", reuse_job)
        self.assertIn("--parent-equivalence", reuse_job)
        self.assertIn("github.event_name == 'push' && github.ref == 'refs/heads/main'", reuse_job)
        self.assertIn("python3 -B scripts/ci_tree_reuse.py", reuse_job)
        test = self.job("test", "dispatch-measurement")
        self.assertIn("needs: reuse", test)
        self.assertIn("${{ !cancelled() && !inputs.dispatch_measurement && needs.reuse.outputs.run_id == '' }}", test)

    def test_coverage_runs_whenever_the_tests_ran_even_though_reuse_was_skipped(self):
        # A skipped needed job skips its dependents unless their condition checks status.
        coverage = self.job("coverage", "deny")
        self.assertIn("needs: [test, deployment-backup]", coverage)
        self.assertIn("${{ !cancelled() && !inputs.dispatch_measurement && needs.test.result == 'success' && needs.deployment-backup.result == 'success' && (vars.TT_CI_DEDICATED != 'true' || github.event_name == 'schedule' || inputs.coverage) }}", coverage)

    def test_dedicated_runner_measures_coverage_only_nightly_or_on_request(self):
        test = self.job("test", "dispatch-measurement")
        run = test.split("Run individually scheduled tests", 1)[1].split("- name:", 1)[0]
        self.assertIn("(github.event_name == 'schedule' || inputs.coverage) && 'true' || 'false'", run)
        self.assertIn("flag=--no-coverage", run)
        self.assertIn("cron:", self.text.split("jobs:", 1)[0])
        publish = test.split("Publish coverage from this execution", 1)[1].split("- name:", 1)[0]
        self.assertIn("matrix.slots == 0 || github.event_name == 'schedule' || inputs.coverage", publish)

    def test_only_web_changes_skip_this_workflow_on_pull_requests(self):
        trigger = self.text.split("jobs:", 1)[0]
        pull_request = trigger.split("  pull_request:\n", 1)[1].split("\n  schedule:", 1)[0]
        self.assertEqual([line.strip() for line in pull_request.splitlines() if line.strip().startswith("- ")],
                         ["- 'web/**'"])
        self.assertNotIn("paths", trigger.split("  pull_request:", 1)[0])

    def test_pull_requests_record_the_tested_tree_after_all_test_steps(self):
        test = self.job("test", "dispatch-measurement")
        record = test.split("Record the tested tree", 1)[1]
        self.assertIn("github.event_name == 'pull_request' && matrix.shard == 1", record)
        self.assertIn("name: tested-tree-${{ steps.tree.outputs.tree }}", record)
        self.assertLess(test.index("Run individually scheduled tests"), test.index("Record the tested tree"))


if __name__ == "__main__":
    unittest.main()
