"""Find a successful pull request CI run that tested exactly this tree.

The policy and its limits are recorded in docs/adr/0082-main-test-reuse.md.
"""
import argparse
import json
import os
import re
import sys
from urllib.parse import urlencode
from urllib.request import Request, urlopen


TREE = re.compile(r"[0-9a-f]{40}")
WORKFLOW = ".github/workflows/ci.yml"


def _passed(run):
    same_repository = (run.get("head_repository") or {}).get("id") == (run.get("repository") or {}).get("id")
    return (run.get("path") == WORKFLOW and run.get("event") == "pull_request"
            and run.get("status") == "completed" and run.get("conclusion") == "success"
            and same_repository)


def find(tree, github):
    """Return the run that recorded this tree after all of its tests passed."""
    if not isinstance(tree, str) or not TREE.fullmatch(tree):
        raise ValueError("tree must be a full lowercase object name")
    name = f"tested-tree-{tree}"
    artifacts = github.get("actions/artifacts", name=name, per_page=30)["artifacts"]
    for artifact in artifacts:
        if artifact.get("name") != name or artifact.get("expired"):
            continue
        run_id = (artifact.get("workflow_run") or {}).get("id")
        if type(run_id) is not int:
            continue
        if _passed(github.get(f"actions/runs/{run_id}")):
            return run_id
    return None


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
    args = parser.parse_args()
    github = GitHub(args.repository, os.environ["GITHUB_TOKEN"],
                    os.environ.get("GITHUB_API_URL", "https://api.github.com"))
    try:
        run_id = find(args.tree, github)
    except Exception as error:
        # Missing evidence only costs a full test run; never treat it as reuse.
        print(f"test reuse unavailable: {type(error).__name__}", file=sys.stderr)
        run_id = None
    print(json.dumps({"tree": args.tree, "reused_run": run_id}), file=sys.stderr)
    print("\n".join(output_lines(run_id)))


if __name__ == "__main__":
    main()
