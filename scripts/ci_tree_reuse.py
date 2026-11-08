"""Find a successful pull request run that tested exactly this tree.

The policy and its limits are recorded in docs/adr/0082-main-test-reuse.md;
docs/adr/0088-ci-time-budget.md extends it to the browser suites.
"""
import argparse
import json
import os
import re
import subprocess
import sys
from urllib.parse import urlencode
from urllib.request import Request, urlopen


TREE = re.compile(r"[0-9a-f]{40}")
# Each kind names the workflow that ran the tests and the artifact it records.
KINDS = {
    "rust": (".github/workflows/ci.yml", "tested-tree"),
    "web": (".github/workflows/web.yml", "tested-web-tree"),
}


def _passed(run, workflow):
    same_repository = (run.get("head_repository") or {}).get("id") == (run.get("repository") or {}).get("id")
    return (run.get("path") == workflow and run.get("event") == "pull_request"
            and run.get("status") == "completed" and run.get("conclusion") == "success"
            and same_repository)


def find(tree, github, kind="rust"):
    """Return the run that recorded this tree after all of its tests passed."""
    if not isinstance(tree, str) or not TREE.fullmatch(tree):
        raise ValueError("tree must be a full lowercase object name")
    if kind not in KINDS:
        raise ValueError("unknown test kind")
    workflow, prefix = KINDS[kind]
    name = f"{prefix}-{tree}"
    artifacts = github.get("actions/artifacts", name=name, per_page=30)["artifacts"]
    for artifact in artifacts:
        if artifact.get("name") != name or artifact.get("expired"):
            continue
        run_id = (artifact.get("workflow_run") or {}).get("id")
        if type(run_id) is not int:
            continue
        if _passed(github.get(f"actions/runs/{run_id}"), workflow):
            return run_id
    return None


# ci.yml ignores pull requests that change only this directory.
IGNORED_PREFIX = "web/"


def equivalent_parent(parent, changed, github):
    """Return the passed main run of the parent when only ignored paths changed."""
    if not isinstance(parent, str) or not TREE.fullmatch(parent):
        raise ValueError("parent must be a full lowercase commit name")
    if not changed or not all(path.startswith(IGNORED_PREFIX) for path in changed):
        return None
    workflow = KINDS["rust"][0]
    runs = github.get(f"actions/workflows/{workflow.rsplit('/', 1)[1]}/runs", branch="main",
                      event="push", head_sha=parent, status="success", per_page=30)
    for run in runs.get("workflow_runs", []):
        if (run.get("head_sha") == parent and run.get("path") == workflow
                and run.get("event") == "push" and run.get("head_branch") == "main"
                and run.get("status") == "completed" and run.get("conclusion") == "success"
                and type(run.get("id")) is int):
            return run["id"]
    return None


def _git(*arguments):
    return subprocess.run(["git", *arguments], check=True, capture_output=True, text=True).stdout


def output_lines(run_id):
    return [f"run_id={run_id or ''}"]


class GitHub:
    def __init__(self, repository, token, api="https://api.github.com"):
        self.base = f"{api}/repos/{repository}"
        self.token = token

    def get(self, path, **query):
        suffix = f"?{urlencode(query)}" if query else ""
        request = Request(f"{self.base}/{path}{suffix}", headers={
            "Authorization": f"Bearer {self.token}", "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28"})
        with urlopen(request, timeout=20) as response:
            return json.load(response)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tree", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--kind", choices=sorted(KINDS), default="rust")
    parser.add_argument("--parent-equivalence", action="store_true",
                        help="also accept the parent's main run when only web/ changed")
    args = parser.parse_args()
    github = GitHub(args.repository, os.environ["GITHUB_TOKEN"],
                    os.environ.get("GITHUB_API_URL", "https://api.github.com"))
    try:
        run_id = find(args.tree, github, args.kind)
        if run_id is None and args.parent_equivalence:
            parent = _git("rev-parse", "HEAD^").strip()
            changed = _git("diff", "--name-only", "HEAD^", "HEAD").split()
            run_id = equivalent_parent(parent, changed, github)
    except Exception as error:
        # Missing evidence only costs a full test run; never treat it as reuse.
        print(f"test reuse unavailable: {type(error).__name__}", file=sys.stderr)
        run_id = None
    print(json.dumps({"kind": args.kind, "tree": args.tree, "reused_run": run_id}), file=sys.stderr)
    print("\n".join(output_lines(run_id)))


if __name__ == "__main__":
    main()
