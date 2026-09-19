# Resources linked to historical resolutions

## Status

Accepted for the domain and application contract. PostgreSQL, HTTP and Qadra
integration and their acceptance remain separate implementation work.

## Context

The scope in `docs/procedural-resources-scope.md` requires resources linked to
resolutions, declared acts, exact evidence and immutable history. A resource is
not a fourth ordinary case stage. Several resources may refer to one resolution;
organizational archiving does not declare withdrawal or another legal effect.

The repository already verifies historical resolutions, directory selections and
exact document versions. Its bounded PDF/DOCX admission processes at most two
distinct documents in one batch. Reusing that policy avoids a second evidence
pipeline and does not establish a legal format requirement.

## Decision

- Give each resource and declared act its own identity. Resource revisions bind
  commands, captured values, sources, actor ID/email, observed administration and
  stage. Register, correct, record/correct an act, archive and reactivate are
  distinct commands. An act change also advances the resource revision. History
  exposes the act recorded by each revision, including earlier corrections.
- Select a resolution by exact revision and capture verified source projections.
  Optional directory references retain exact historical revisions independently
  of declared appellant names and roles. They never grant account access.
- Apply technical bounds of 32 appellants and two evidence entries per act.
  Retain function-specific evidence locators. Admit new exact versions in one
  complete batch; a retained admitted version is checked against its capture,
  not silently readmitted or replaced with its head. Additional formats require
  a separately specified admission policy and tests.
- Use the existing source encodings as bounded components of PRSS1. PRTX1 binds
  the reviewed command, actor email, previous capture, values, source captures,
  act and exact administrative/stage observations. PRCP1 binds the resulting
  submission digest and definitive commit time. Domain values retain PRSC1 and
  acts PRAC1. None of these encodings determines legal applicability.
- Authenticate before work and reauthenticate the complete principal after
  preparation and before submission; queries reauthenticate before disclosure.
  The store rechecks active account, role and current membership under the shared
  audit lock. Owner manages all, assigned Litigator manages, assigned Paralegal
  reads, and Client is denied. Closed cases retain authorized history and block
  new mutations.
- Preparation resolves exact sources without reserving an operation. Commit
  compares resource and act heads, the complete prepared evidence and observed
  administration/stage, then atomically appends revision, receipt and audit.
  Changed observations conflict explicitly; preparation is not a durable lock.
  Reusing an operation with the same actor and command returns its original
  audited receipt, including after closure. Different reuse conflicts. Replay
  still requires current authorization and cannot disclose a foreign resource.
- Lists select current heads before filters and exclusive UUID pagination;
  history is descending and bounded. Read and replay responses leave the store
  only after their read audit commits. Read validation preserves exact historical
  evidence and does not admit documents or infer current legal effects.

## Consequences

The application can be exercised through repository and existing document ports
without claiming that persistence, transport or the complete resource workflow
has been implemented. Audiences, linked calculated terms, alerts and a qualified
legal corpus remain required by `docs/procedural-resources-scope.md`; this contract
does not replace them with manual dates. No command advances a case stage,
suspends a deadline, certifies filing, admissibility or finality, or changes the
approved academic objectives.
