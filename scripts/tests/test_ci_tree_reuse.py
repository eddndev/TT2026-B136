"""A main push reuses tests only from a successful pull request run of the same tree."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ci_tree_reuse as reuse


ROOT = Path(__file__).resolve().parents[2]
TREE = "e" * 40
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

    def test_tree_must_be_a_full_object_name(self):
        for tree in ("E" * 40, "e" * 39, "", "e" * 40 + "\n"):
            with self.subTest(tree=tree), self.assertRaises(ValueError):
                reuse.find(tree, FakeGitHub([], {}))

    def test_outputs_leave_the_run_empty_without_evidence(self):
        self.assertEqual(reuse.output_lines(7), ["run_id=7"])
        self.assertEqual(reuse.output_lines(None), ["run_id="])


class WorkflowTests(unittest.TestCase):
    text = (ROOT / ".github/workflows/ci.yml").read_text()

    def job(self, name, following):
        return self.text.split(f"  {name}:\n", 1)[1].split(f"\n  {following}:\n", 1)[0]

    def test_reuse_runs_only_for_main_pushes_and_tests_run_without_evidence(self):
        reuse_job = self.job("reuse", "test")
        self.assertIn("github.event_name == 'push' && github.ref == 'refs/heads/main'", reuse_job)
        self.assertIn("python3 -B scripts/ci_tree_reuse.py", reuse_job)
        test = self.job("test", "dispatch-measurement")
        self.assertIn("needs: reuse", test)
        self.assertIn("${{ !cancelled() && !inputs.dispatch_measurement && needs.reuse.outputs.run_id == '' }}", test)

    def test_pull_requests_record_the_tested_tree_after_all_test_steps(self):
        test = self.job("test", "dispatch-measurement")
        record = test.split("Record the tested tree", 1)[1]
        self.assertIn("github.event_name == 'pull_request' && matrix.shard == 1", record)
        self.assertIn("name: tested-tree-${{ steps.tree.outputs.tree }}", record)
        self.assertLess(test.index("Run individually scheduled tests"), test.index("Record the tested tree"))


if __name__ == "__main__":
    unittest.main()
