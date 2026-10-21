"""Find gate runs that already verified the exact tree a deploy run packages.

The policy and its limits are recorded in docs/adr/0081-deploy-gate-reuse.md.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import urlencode
from urllib.request import Request, urlopen


ROOT = Path(__file__).resolve().parents[1]
WEB_WORKFLOW = ".github/workflows/web.yml"
MODULE_ARTIFACT = "browser-crypto-module"
SHA = re.compile(r"[0-9a-f]{40}")
UNSUPPORTED = re.compile(r"[?\[\]{}!]")


def push_paths(text):
    """Return the push path filters of a workflow written in this repository's style."""
    lines = text.splitlines()
    try:
        start = lines.index("on:")
        push = lines.index("  push:", start)
        paths = lines.index("    paths:", push)
    except ValueError as error:
        raise ValueError("workflow push path filters are missing") from error
    patterns = []
    for line in lines[paths + 1:]:
        match = re.fullmatch(r"      - '([^']+)'", line)
        if match is None:
            break
        patterns.append(match.group(1))
    if not patterns:
        raise ValueError("workflow push path filters are empty")
    return patterns


def _regex(pattern):
    if UNSUPPORTED.search(pattern):
        raise ValueError("unsupported path filter syntax")
    parts = []
    for token in re.split(r"(\*\*|\*)", pattern):
        if token == "**":
            parts.append(".*")
        elif token == "*":
            parts.append("[^/]*")
        else:
            parts.append(re.escape(token))
    return re.compile("".join(parts))


def matches(path, patterns):
    return any(_regex(pattern).fullmatch(path) for pattern in patterns)


def _verified(run_, sha):
    return (run_.get("head_sha") == sha and run_.get("event") == "push"
            and run_.get("head_branch") == "main" and run_.get("status") == "completed"
            and run_.get("conclusion") == "success")


def exact_run(runs, commit):
    """Return a successful main push run of exactly this commit."""
    for run_ in runs:
        if _verified(run_, commit):
            return run_["id"]
    return None


def equivalent_run(runs, commit, patterns, git, artifacts):
    """Return a successful main push run whose gated inputs equal this commit's."""
    for run_ in runs:
        earlier = run_.get("head_sha")
        if not isinstance(earlier, str) or not SHA.fullmatch(earlier) or not _verified(run_, earlier):
            continue
        if not git.is_ancestor(earlier, commit):
            continue
        if git.blob(earlier, WEB_WORKFLOW) != git.blob(commit, WEB_WORKFLOW):
            continue
        if any(matches(path, patterns) for path in git.changed_paths(earlier, commit)):
            continue
        if any(item.get("name") == MODULE_ARTIFACT and not item.get("expired")
               for item in artifacts(run_["id"])):
            return run_["id"]
    return None


def output_lines(decision):
    rust, web = decision["rust_run"], decision["web_run"]
    return [f"reuse_rust={'true' if rust else 'false'}", f"rust_run_id={rust or ''}",
            f"reuse_web={'true' if web else 'false'}", f"web_run_id={web or ''}"]


class Git:
    def _run(self, *arguments, check=True):
        return subprocess.run(["git", *arguments], cwd=ROOT, check=check,
                              capture_output=True, text=True)

    def is_ancestor(self, older, newer):
        return self._run("merge-base", "--is-ancestor", older, newer, check=False).returncode == 0

    def changed_paths(self, older, newer):
        return self._run("diff", "--name-only", older, newer).stdout.split()

    def blob(self, revision, path):
        return self._run("rev-parse", f"{revision}:{path}").stdout.strip()


class GitHub:
    def __init__(self, repository, token, api="https://api.github.com"):
        self.base = f"{api}/repos/{repository}"
        self.token = token

    def get(self, path, **query):
        request = Request(f"{self.base}/{path}?{urlencode(query)}", headers={
            "Authorization": f"Bearer {self.token}", "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28"})
        with urlopen(request, timeout=20) as response:
            return json.load(response)

    def runs(self, workflow, **query):
        return self.get(f"actions/workflows/{workflow}/runs", branch="main", event="push",
                        status="success", per_page=30, **query)["workflow_runs"]

    def artifacts(self, run_id):
        return self.get(f"actions/runs/{run_id}/artifacts", per_page=100)["artifacts"]


def decide(commit, github, git):
    if not SHA.fullmatch(commit):
        raise ValueError("deploy commit must be a full SHA")
    rust = exact_run(github.runs("ci.yml", head_sha=commit), commit)
    patterns = push_paths((ROOT / WEB_WORKFLOW).read_text())
    web = equivalent_run(github.runs("web.yml"), commit, patterns, git, github.artifacts)
    return {"commit": commit, "rust_run": rust, "web_run": web}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--repository", required=True)
    args = parser.parse_args()
    github = GitHub(args.repository, os.environ["GITHUB_TOKEN"],
                    os.environ.get("GITHUB_API_URL", "https://api.github.com"))
    try:
        decision = decide(args.commit, github, Git())
    except Exception as error:
        # Missing evidence only costs a full gate run; never treat it as reuse.
        print(f"gate reuse unavailable: {type(error).__name__}", file=sys.stderr)
        decision = {"commit": args.commit, "rust_run": None, "web_run": None}
    print(json.dumps(decision, sort_keys=True), file=sys.stderr)
    print("\n".join(output_lines(decision)))


if __name__ == "__main__":
    main()
