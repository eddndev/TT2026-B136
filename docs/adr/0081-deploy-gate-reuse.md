# ADR 0081: Reuse verified gates when packaging a version

## Status

Accepted on 2026-10-21 at the author's request.

## Context

`deploy.yml` called the CI and Web workflows before packaging every candidate,
even when the same commit had already passed CI on `main` and Web had verified
identical inputs. Preparing `v0.1.13` repeated about 18 minutes of Rust tests
and the browser campaign that had already passed, delaying each deployment
window by roughly half an hour without adding evidence.

## Decision

A `gates` job runs `scripts/deploy_gate_reuse.py` before packaging:

- CI is reused only when a completed, successful `push` run on `main` exists for
  exactly the packaged commit.
- Web is reused when a successful `push` run on `main` verified an ancestor of
  the packaged commit, `web.yml` is identical in both revisions, no path changed
  between them matches its push filters, and that run still holds an unexpired
  `browser-crypto-module` artifact. Packaging downloads that exact artifact.
- Filters use only `*` and `**`; any other syntax, missing evidence or API error
  disables reuse, and the corresponding workflow runs as before.

Packaging requires each gate to have succeeded in the same run or to have been
skipped because reuse was proven.

## Consequences

Preparing a candidate after a green `main` campaign only builds and uploads the
package. A deployment run can no longer catch a flaky failure that did not
appear in the reused run; the reused run remains linked from the job output.
Web reuse ends when the browser module artifact expires after seven days.

## Amendment of 2026-10-28

The Web workflow uploads a second module artifact, `evidence-verify-module`,
built by `scripts/build-evidence-verify.sh` for the evidence validator of
`docs/adr/0085-in-browser-evidence-validation.md`. Web reuse now requires that
run to hold both `browser-crypto-module` and `evidence-verify-module`
unexpired, and packaging downloads both from it. Runs made before the second
artifact existed are not equivalent, so the first deployment after this change
runs the Web gate again.
