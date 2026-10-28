"""Deploy packaging reuses only exact, unexpired gate evidence."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import deploy_gate_reuse as reuse


ROOT = Path(__file__).resolve().parents[2]
COMMIT = "a" * 40
EARLIER = "b" * 40
MODULES = ("browser-crypto-module", "evidence-verify-module")


def run(identifier, sha, *, event="push", branch="main", conclusion="success", path=None):
    return {"id": identifier, "head_sha": sha, "event": event, "head_branch": branch,
            "status": "completed", "conclusion": conclusion, "path": path}


class FakeGit:
    def __init__(self, *, ancestors=(EARLIER,), changed=(), blobs=None):
        self.ancestors = set(ancestors)
        self.changed = list(changed)
        self.blobs = blobs or {}

    def is_ancestor(self, older, newer):
        return newer == COMMIT and older in self.ancestors | {COMMIT}

    def changed_paths(self, older, newer):
        return [] if older == newer else list(self.changed)

    def blob(self, revision, path):
        return self.blobs.get((revision, path), "same-blob")


class PatternTests(unittest.TestCase):
    def test_web_push_paths_are_read_from_the_real_workflow(self):
        patterns = reuse.push_paths((ROOT / ".github/workflows/web.yml").read_text())
        self.assertIn("web/**", patterns)
        self.assertIn("crates/**", patterns)
        self.assertIn(".github/workflows/web.yml", patterns)
        try:
            import yaml
        except ImportError:
            return
        parsed = yaml.safe_load((ROOT / ".github/workflows/web.yml").read_text())
        self.assertEqual(patterns, parsed[True]["push"]["paths"])

    def test_globs_follow_single_and_double_star_semantics(self):
        patterns = ["web/**", "scripts/*browser-crypto*", "Cargo.*"]
        for path, expected in (("web/apps/qadra/x.svelte", True), ("scripts/build-browser-crypto.sh", True),
                               ("scripts/sub/build-browser-crypto.sh", False), ("Cargo.lock", True),
                               ("crates/Cargo.toml", False), ("docs/web/x.md", False)):
            with self.subTest(path=path):
                self.assertEqual(reuse.matches(path, patterns), expected)

    def test_unsupported_glob_syntax_fails_closed(self):
        for pattern in ("web/?", "web/[ab]", "!web/**", "web/{a,b}"):
            with self.subTest(pattern=pattern), self.assertRaises(ValueError):
                reuse.matches("web/a", [pattern])

    def test_unexpected_workflow_shape_fails_closed(self):
        with self.assertRaises(ValueError):
            reuse.push_paths("on:\n  push:\n    branches: [main]\n")


class RustReuseTests(unittest.TestCase):
    def test_only_a_successful_main_push_for_the_exact_commit_is_reused(self):
        runs = [run(1, COMMIT, event="pull_request"), run(2, COMMIT, branch="other"),
                run(3, COMMIT, conclusion="cancelled"), run(4, EARLIER), run(5, COMMIT)]
        self.assertEqual(reuse.exact_run(runs, COMMIT), 5)

    def test_no_matching_run_means_the_gate_runs_again(self):
        self.assertIsNone(reuse.exact_run([run(1, EARLIER), run(2, COMMIT, conclusion="failure")], COMMIT))


class WebReuseTests(unittest.TestCase):
    patterns = ["web/**", "crates/**", ".github/workflows/web.yml"]

    def choose(self, runs, git, artifacts=None):
        artifacts = artifacts if artifacts is not None else {
            run_["id"]: [{"name": name, "expired": False} for name in MODULES] for run_ in runs}
        return reuse.equivalent_run(runs, COMMIT, self.patterns, git, lambda i: artifacts.get(i, []))

    def test_earlier_run_is_reused_when_no_gated_path_changed(self):
        git = FakeGit(changed=["docs/x.md", "ops/deploy/renew_crl.py"])
        self.assertEqual(self.choose([run(7, EARLIER)], git), 7)

    def test_a_changed_gated_path_prevents_reuse(self):
        git = FakeGit(changed=["docs/x.md", "crates/web/src/lib.rs"])
        self.assertIsNone(self.choose([run(7, EARLIER)], git))

    def test_runs_that_are_not_ancestors_or_have_another_workflow_are_ignored(self):
        self.assertIsNone(self.choose([run(7, "c" * 40)], FakeGit()))
        git = FakeGit(blobs={(EARLIER, ".github/workflows/web.yml"): "older-blob"})
        self.assertIsNone(self.choose([run(7, EARLIER)], git))

    def test_expired_or_missing_browser_module_prevents_reuse(self):
        for artifacts in ({7: [{"name": "browser-crypto-module", "expired": True}]}, {7: []}):
            with self.subTest(artifacts=artifacts):
                self.assertIsNone(self.choose([run(7, EARLIER)], FakeGit(), artifacts))

    def test_a_run_missing_either_browser_module_is_not_equivalent(self):
        for kept in MODULES:
            with self.subTest(kept=kept):
                artifacts = {7: [{"name": kept, "expired": False}]}
                self.assertIsNone(self.choose([run(7, EARLIER)], FakeGit(), artifacts))
        expired = {7: [{"name": "browser-crypto-module", "expired": False},
                       {"name": "evidence-verify-module", "expired": True}]}
        self.assertIsNone(self.choose([run(7, EARLIER)], FakeGit(), expired))

    def test_unsuccessful_or_non_main_runs_are_not_evidence(self):
        runs = [run(7, EARLIER, conclusion="failure"), run(8, EARLIER, event="pull_request")]
        self.assertIsNone(self.choose(runs, FakeGit()))


class DecisionTests(unittest.TestCase):
    def test_outputs_are_explicit_strings_for_both_gates(self):
        lines = reuse.output_lines({"rust_run": 5, "web_run": None})
        self.assertEqual(lines, ["reuse_rust=true", "rust_run_id=5", "reuse_web=false", "web_run_id="])


if __name__ == "__main__":
    unittest.main()
