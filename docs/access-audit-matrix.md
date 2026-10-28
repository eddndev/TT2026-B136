# Access audit matrix and acceptance protocol

## Status and boundary

This is a finite implementation and acceptance inventory derived from the local
versioned routers and their existing contracts on 2026-10-11. It defines
acceptance for the local implementation of
[durable access context](adr/0077-durable-access-audit-context.md). Functional
criteria 1-7 below are accepted locally from the combined focused, native,
composed API/restore and real Qadra evidence. The reconciled HTTP measurement
contains 2347 durable records across 207 of the 222 functional pairs; the exact
15 unobserved pairs and their existing fixture mappings are retained below.
On 2026-10-28 the catalog grew to 223 pairs with
`GET /api/v1/evidence/trust`; the 207/222 HTTP measurement and the 222 native
route payloads below remain historical figures over the earlier catalog, and
the new pair has no durable-latency sample of its own.
This does not establish HTTP timing for every row, an exhaustive per-role
campaign, integration or deployment. Contracts and the 414-page manuscript are reconciled and locally inspected;
exact-revision CI/integration gates are tracked separately in criterion 8. Existing workflow
acceptance remains separate in [verification-report.md](verification-report.md).
The parent scope is [technical-closure.md](technical-closure.md): RF-02, CU-04,
CU-17 and RF-20 in [analysis and design](../latex/chapters/03-analisis-diseno.tex).

The maximum configured product surface contains **223 functional method/path
pairs on 191 distinct normalized route patterns**: 115 GET, 88 POST, 19 PUT and
one DELETE. Two additional explicit HEAD handlers reject with 405. The 192
route declarations include separate GET and POST declarations for `/api/v1/users`.
The appendix expands the two procedural-fact families and both deadline-profile
collections. Path parameters retain the names used by each source router.

This is a source catalog, not a production endpoint census. Certificate-login
`start` and `proof` are absent when that optional workflow is disabled; its
`availability` route remains present. The nine individual-signing routes are
composed only when signing components are present. Declare enabled components in
each campaign; disabled operations retain their existing unavailable behavior.
`GET /healthz`, static assets and the reverse proxy are outside the product
operation count. Implicit HEAD handling inherited from GET is a transport case,
not another business operation; it must not become an undocumented bypass.

## Authorization scenarios

Every appendix row names one policy group below. These groups summarize current
contracts; they do not replace application/store authorization or grant new
permissions. Use valid fixtures for the positive role and the same operation
with the rejected role/resource for the negative scenario. Authentication and
resource checks remain delegated to the existing workflow. All private groups
also require missing/revoked/expired-session rejection and current-account
revalidation; no token, password, MFA code or private capability is recorded.

| Policy group | Authorized scenario | Rejection or scope scenario | Contract |
| --- | --- | --- | --- |
| bootstrap | No users exist; create first Owner using existing enrollment. | A later bootstrap is closed; a claimed UUID is not an authenticated principal. | [HTTP](http-api.md) |
| password-login | Correct credentials create the existing MFA challenge. | Invalid credentials, inactive account and lock limits retain generic public results; no session is inferred from first-factor success. | [HTTP](http-api.md) |
| mfa | Valid server challenge and TOTP/recovery produce the existing session. | Invalid/expired/consumed challenge, invalid/replayed TOTP, consumed recovery and operational failure stay distinct where already known. | [MFA](mfa-access-logs.md) |
| session | Current bearer for any of the four roles reads identity/session, records activity, logs out or reads the current evidence trust anchors. | Missing, revoked or expired session; record activity cannot revive it. | [HTTP](http-api.md) |
| owner-members | Active Owner lists, creates, reads and changes accounts/assignments. | Litigator, Paralegal and Client rejected; last active Owner and revision guards remain. | [Members](members-api.md), [HTTP](http-api.md) |
| basic-cases | Owner sees all; assigned Litigator, Paralegal and Client read basic metadata. Owner/Litigator create; only Owner assigns/removes members. | Unassigned account cannot learn an exact case; Paralegal/Client cannot create; Litigator cannot manage members. | [HTTP](http-api.md) |
| case-administration | Owner and Litigator create penal cases; Owner manages all, assigned Litigator manages, assigned Paralegal reads. | Client denied even if assigned; unassigned staff denied/hidden under current contract. | [Administration](case-administration-api.md) |
| documents | Owner globally, assigned Litigator/Paralegal read, upload, classify, append, verify and export. Only Owner/Litigator use existing server sealing. | Client denied even if assigned; Paralegal cannot seal; a document/case mismatch remains hidden even to Owner. | [HTTP document matrix](http-api.md), [Content](document-content-api.md) |
| staff-case | Owner manages globally; assigned Litigator manages; assigned Paralegal reads. Read-only history/receipts survive closure where the family contract allows. | Client denied; Paralegal cannot prepare/submit/mutate; revoked membership blocks later access; closed cases block mutations. | Family contracts listed below |
| global-calendars | Owner publishes/replaces/retires; Owner, Litigator and Paralegal read without membership. | Client denied; non-Owner mutations rejected. | [Calendars](judicial-calendars-api.md) |
| deadline-profiles | Owner manages both collections; staff reads global profiles and authorized case profiles. | Client denied; non-Owner mutations rejected; case collections require current membership for Litigator/Paralegal. | [Profiles](deadline-profiles-api.md) |
| staff-agenda | Owner sees authorized office scope; Litigator/Paralegal see assigned cases. | Client denied; out-of-scope rows excluded; empty authorized selections remain valid reads. | [Agenda](agenda-api.md), [Hearings](hearings-api.md) |
| personal-alerts | Owner/Litigator/Paralegal access only their own preferences and inbox. | Client and another account's alert denied; Owner is not a global inbox reader. | [Alerts](alerts-api.md) |
| dashboard | Owner office aggregate or Litigator assigned-case aggregate. | Paralegal/Client denied, including an otherwise empty aggregate. | [Dashboard](dashboard-api.md) |
| own-reports | Owner/Litigator request and read their own reports within current full capture authorization. | Paralegal/Client denied; another requester denied even if Owner; revoked capture access blocks download. | [Reports](case-reports-api.md) |
| owner-audit | Current active Owner queries recorded events or verifies the chain. | Litigator/Paralegal/Client denied; revocation suppresses disclosure, including empty pages. | [Audit query](audit-events-api.md), [HTTP](http-api.md) |
| owner-integrity | Current Owner reads the integrity-incident inbox. | All other roles denied; no content access is granted by the incident record. | [Content and incidents](document-content-api.md) |
| owner-binding | Current Owner operates only on its own certificate binding. | Other roles and another account's binding rejected under current contract. | [Owner certificates](http-owner-certificates.md) |
| certificate-login | Availability is public; enabled first-factor workflow checks the declared Owner binding/proof before MFA. | Disabled start/proof remain absent; rejected proof does not expose account/binding existence; first factor is not a session. | [Certificate login](owner-certificate-login-http.md) |
| litigator-binding | Current Litigator manages its own public signing certificate and possession proof. | Owner/Paralegal/Client cannot substitute the holder; another account's binding rejected. | [Individual signing](http-individual-signing.md) |
| individual-signing | Assigned Litigator prepares/submits exact-version signatures; authorized receipt recovery survives case closure; verification uses existing VerifyDocument roles. | Wrong account/version, lost membership, withdrawn binding or invalid trust rejected; Client remains denied. | [Individual signing](http-individual-signing.md) |
| owner-invitations | Current Owner invites Litigator/Paralegal and reads/resends existing invitations. | Non-Owner rejected; unsupported invitation roles and existing-account constraints preserved. | [Invitations](member-invitations-api.md) |
| invitation-recipient | Public private-capability workflow prepares/finishes enrollment and reconciles the exact receipt. | Invalid/expired capability, wrong password/code and conflicting operation retain existing rejection; no unrelated bearer is adopted. | [Invitations](member-invitations-api.md) |
| password-reset | Existing public request/completion workflow with generic public responses and private reset capability. | Invalid/expired/used capability and operational errors retain existing semantics; generic request acceptance does not prove account existence or email delivery. | [Reset](password-reset-public.md) |
| head-rejected | No positive operation: explicit content/report HEAD is 405. | Retain 405 and audit transport outcome without emitting a business read/download action. | Source handlers in appendix |

The `staff-case` group covers [participants and identities](typed-participants-api.md),
[stages](case-stages-api.md), [hearings](hearings-api.md),
[results](hearing-results-api.md), [facts](procedural-facts-api.md),
[resources](procedural-resources-api.md), [associations](resource-activities-api.md),
[resource deadlines](resource-deadlines-api.md), [resource hearings](resource-hearings.md),
[derived deadlines](hearing-derived-deadlines.md), [deadlines](deadlines-api.md),
[precautionary hearings](precautionary-hearings-api.md),
[measure decisions](measure-decisions-api.md) and [measure records](measure-records-api.md).
The manual participant directory is specified in [HTTP](http-api.md).
Preparation is a management operation even when it does not persist a business
record. Do not infer authorization from the HTTP method alone.

Client documentary access remains an explicit human policy decision in
[technical-closure.md](technical-closure.md). This matrix preserves its current
denial and the separate assigned basic-case metadata permission. It does not
open document access, add roles, or alter the finite workflow catalogs.

## Capture and trust contract

Use one server-generated request identity, canonical recognized route, method,
source provenance and the verified account observation shared across the HTTP
operation. An observed MFA/login target UUID is not automatically an authenticated
principal. Preserve unknown identity explicitly; never infer a historical UUID
from the current owner of an email. HTTP, CLI and internal consumers have distinct
origins; CLI/internal activity has no fabricated HTTP source IP.

Keep existing business events atomic with their business transactions. A separate
`access.*` event represents one request outcome and is not another upload, legal
act or attended deadline for activity counters. Preserve original chained bytes
for legacy events; version and hash new metadata. Query/restore must retain both
representations without historical backfill. The existing Owner-only query and
append-only protections remain. MFA diagnostic logs remain a separate facility.

The admitted handler result maps to `access.granted` for 2xx, `access.denied`
for 401/403, `access.failed` for 5xx, and `access.rejected` for the remaining
responses. A hidden-or-missing 404 remains ambiguous; 409/422 do not alone prove
permission denial. A public 2xx does not prove account existence or mail delivery.
Never record adapter error text or the request body/query. A 503 returned before
request admission is an operational capacity rejection outside the executed
operation population; existing operational MFA diagnostics remain separate.
An admitted owner retains its request permit through handler completion and
durable append even if the client disconnects. Orderly shutdown must also drain
those owners; shutdown acceptance is separate from client-cancellation acceptance.
If access append fails, the generic 500 replaces the handler response. This is
not proof that the business operation rolled back: preserve existing receipts
and explicit uncertain-response reconciliation without automatic retry.

Selected source policy, with operational acceptance still pending:

- Default to the socket peer address; ignore forwarded headers in direct mode.
- Enable trusted-loopback-proxy mode only through explicit server configuration.
  In that mode, accept a forwarded origin only from a loopback socket peer and
  only as one scalar `X-Forwarded-For` IP address, with no duplicate header or
  comma-separated chain. Invalid, duplicate or missing forwarded values fall back
  to the socket peer. Missing socket metadata remains unavailable. No header is
  trusted when the observed peer is not loopback.
- The versioned Nginx template in [ops/deploy/host.py](../ops/deploy/host.py)
  overwrites `X-Forwarded-For` with `$remote_addr`. Preserve that overwrite and
  verify direct/header-spoofing, untrusted peer, duplicate, malformed and valid
  scalar cases. Loopback mode assumes local callers are trusted; do not silently
  enable it for every installation or treat a header as proof by itself.
- Socket metadata enters at [serve_start.rs](../crates/bin/src/serve_start.rs).
  Shared admission and blocking-context propagation belong to
  [runtime.rs](../crates/web/src/runtime.rs); identity observations come from
  [session admission](../crates/application/src/identity/service/session.rs).
  The focused evidence below verifies these implementation boundaries; it does
  not attest the configuration of a deployed proxy.

The event API returns `context: null` for legacy rows or the exact object with
`user_id`, `ip_address`, `source` and `request_id`. The existing `actor` filter
matches exact captured actor text or exact `context.user_id`, without an account
lookup or a new query parameter. The page's 262144-byte text budget includes
compact JSON context bytes in addition to actor/action/resource; null adds zero.
Identity, IP, origin and access result are displayed separately; absent historical
metadata and business actions without an access outcome remain unavailable.

## Durable access-record measurement protocol

For each executed functional method/path and applicable role/resource scenario,
measure the new access record from immediately before waiting for its sink to
successful durable database commit. Include sink scheduling/admission, lock/pool
waiting, append and commit; capture a monotonic start/end. This is access-record
latency, not entire HTTP latency, business computation, browser time, or the time
taken by a verification command. Timing `append_transaction` alone is insufficient
because its caller commits later. Do not claim this measures every earlier
business-event transaction merely because both use the same audit chain.

Before a campaign, declare revision, enabled components, OS, CPU, memory, disk,
PostgreSQL configuration, build profile, history row count/size, concurrent
requests/writers, fixture preparation, warm/cold conditions and exact sample
counts/repetitions. Seed representative populated history, preserve the exact
preexisting chain and enumerate the observed route/scenario coverage. Run the
campaign under one coordinator; do not overlap verification suites/services.

Retain each sample with operation/scenario, request/event reference, source mode,
observed result, start/end or elapsed duration, commit status and failure category.
Publish sample count, individual durations, maximum and a distribution summary;
state the quantile method for any reported percentile. Record incomplete/failed
commits separately: they are not fast successful samples. Retain records after
restart/restoration and independently verify the new and historical chain.

Compare each successful durable sample with the strict `<500 ms` commitment.
Do not substitute an average or p95 threshold: neither is specified by RF-20.
Any violation remains visible and prevents claiming the tested scenario meets
the bound. A finite observed campaign supports only its declared environment and
coverage, not a universal worst-case guarantee. The recorded results below
identify the actual populations and their limits. Native sink measurements and
HTTP measurements are separate populations; only a reconciled HTTP receipt can
identify which enabled operations actually ran through HTTP and what they took.

## Reproducible coverage map

Every functional appendix row resolves to three separate checks: its exact
method/path in **B**, its `policy` in the table below, and its source-family
fixture where a policy spans multiple routers. The 24 policy counts sum to
223. The 112 `staff-case` rows are expanded by exact source below; no appendix
row is added or removed by this mapping. The two HEAD rows resolve separately.

**B - common access boundary:**
[`catalog_tests.rs`](../crates/web/src/access_audit/catalog_tests.rs) consumes
every appendix pair and exercises 200, 401, 403, 404, 422 and 500 with exactly
one outcome and recognized route/resource identity. It includes the two HEAD
patterns for boundary behavior. This is a synthetic router test, not an
execution of the composed workflow, role authorization or actual HEAD handler.
Reproduce it with `cargo test -p web --lib access_audit:: -- --test-threads=1`.

**P - existing policy and transport fixtures:** the following files are
reproducible checks, not a claim that every listed test has been rerun for this
increment. HTTP doubles verify delegation and safe result handling; native API
fixtures exercise the existing application/store permissions. The authorization
scenario table above remains the expected policy. Every row also inherits B.

| Policy (functional pairs) | Existing checks and fixture entry points |
| --- | --- |
| bootstrap (1) | [Auth transport](../crates/web/tests/auth_api.rs), [identity workflow](../crates/application/tests/identity_workflow.rs), [API bootstrap](../scripts/api-demo.sh). |
| password-login (1) | [Auth transport](../crates/web/tests/auth_api.rs), [identity workflow](../crates/application/tests/identity_workflow.rs), [API login](../scripts/api-demo.sh). |
| mfa (2) | [MFA/session cases](../crates/web/tests/auth_support/session_cases.rs), [operational correlation](../crates/web/tests/auth_support/access_log_cases.rs), [API TOTP/recovery/replay](../scripts/api-demo.sh). |
| session (5) | [All-role session cases](../crates/web/tests/auth_support/session_cases.rs), [session policies](../crates/application/tests/identity_support/session_policy_cases.rs), [logout/revoked bearer](../scripts/api-demo.sh), [evidence trust transport](../crates/web/tests/evidence_trust_http.rs), [evidence trust authentication](../crates/application/tests/evidence_trust.rs). |
| owner-members (5) | [Member transport](../crates/web/tests/member_http.rs), [user creation](../crates/web/tests/auth_api.rs), [four-role account/assignment fixture](../scripts/api-members-demo.py). |
| basic-cases (5) | [Case transport](../crates/web/tests/case_api.rs), [four roles, assignment removal and hidden resources](../scripts/api-case-demo.sh). |
| case-administration (6) | [Administration transport](../crates/web/tests/case_administration_api.rs), [penal case and closure fixture](../scripts/api-case-administration-demo.sh). |
| documents (17) | [Current documents](../crates/web/tests/document_api.rs), [queries](../crates/web/tests/document_queries.rs), [versions](../crates/web/tests/document_versions.rs), [metadata](../crates/web/tests/document_metadata.rs), [classified upload](../crates/web/tests/document_metadata_upload.rs), [content](../crates/web/tests/document_content_http.rs). Native fixtures: [role/assignment scope](../scripts/api-case-demo.sh), [versions](../scripts/api-version-demo.sh), [metadata](../scripts/api-metadata-demo.sh), [content](../scripts/api-document-content-demo.py). |
| staff-case (112) | Exact source-family map below. |
| global-calendars (9) | [Calendar transport](../crates/web/tests/judicial_calendar_http.rs), [queries](../crates/web/tests/judicial_calendar_queries.rs), [Owner mutation/staff read/Client denial](../scripts/api-judicial-calendars-demo.py). |
| deadline-profiles (16) | [Both collection routers](../crates/web/tests/deadline_profile_http.rs), [profile roles, case scope and membership loss](../scripts/api-deadline-profiles-demo.py). |
| staff-agenda (2) | [Combined agenda](../crates/web/tests/agenda_http.rs), [hearing collection](../crates/web/src/hearings/route_tests.rs), [agenda scope/Client fixture](../scripts/api-agenda-demo.py), [hearings fixture](../scripts/api-hearings-demo.py). |
| personal-alerts (5) | [Alert transport](../crates/web/tests/alert_http.rs), [own inbox/foreign alert/Client fixture](../scripts/api-alerts-demo.py). |
| dashboard (1) | [Dashboard transport](../crates/web/tests/dashboard_http.rs), [revoked membership and denied roles](../scripts/api-dashboard-demo.py). |
| own-reports (6) | [Report transport](../crates/web/tests/case_reports_http.rs), [activity selector](../crates/web/tests/case_report_activity_http.rs), [requester isolation and lost capture access](../scripts/api-case-reports-demo.py). |
| owner-audit (2) | [Query transport](../crates/web/tests/audit_events_http.rs), [Owner store authorization](../crates/infrastructure/tests/audit_query_postgres.rs), [three denied roles/query/chain/restore](../scripts/api-audit-events-demo.py). |
| owner-integrity (2) | [Incident transport](../crates/web/tests/document_integrity_http.rs), [content and incident fixture](../scripts/api-document-content-demo.py). |
| owner-binding (5) | [Owner certificate transport](../crates/web/tests/owner_certificate_http.rs), [current Owner composition](../crates/web/tests/owner_certificate_composition.rs), [binding fixture](../scripts/api-owner-certificates-demo.py). |
| certificate-login (3) | [Public availability](../crates/web/tests/owner_login_http_cases/availability.rs), [enabled/disabled composition](../crates/web/tests/owner_login_http_cases/composition_cases.rs), [proof transport](../crates/web/tests/owner_login_http_cases/transport.rs), [native login fixture](../scripts/api_owner_login_demo.py). |
| litigator-binding (5) | [Self-bound signing transport](../crates/web/tests/individual_signing_http.rs), [native signing fixture](../scripts/api-individual-signing-demo.py). |
| individual-signing (4) | [Exact-version signing transport](../crates/web/tests/individual_signing_http.rs), [native signing/restore fixture](../scripts/api-individual-signing-demo.py). |
| owner-invitations (4) | [Owner bearer/role delegation](../crates/web/tests/member_invitations.rs), [native invitation acceptance](../crates/bin/src/serve_invitation_acceptance.rs). |
| invitation-recipient (3) | [Public capability/no-session/error contract](../crates/web/tests/member_invitations.rs), [native enrollment/resend/reentry](../crates/bin/src/serve_invitation_acceptance.rs). |
| password-reset (2) | [Generic public request](../crates/web/tests/password_reset_http_support/request_cases.rs), [completion and uncertainty](../crates/web/tests/password_reset_http_support/completion_cases.rs), [native reset/MFA acceptance](../crates/bin/src/serve_password_reset_http_acceptance.rs). |

For `staff-case`, match the exact appendix `source` suffix after
`crates/web/src/`. All methods and paths from that source inherit the indicated
checks, including read/history/receipt and preparation/submission variants.

| Source suffix (pairs) | Transport checks | Existing policy/native fixture |
| --- | --- | --- |
| `case_stages/mod.rs` (4) | [Stages](../crates/web/tests/case_stage_api.rs) | [Stage role/removed-member fixture](../scripts/api-case-stage-demo.sh) |
| `deadlines/mod.rs` (10) | [Deadlines](../crates/web/tests/deadline_http.rs), [responsibles](../crates/web/tests/deadline_responsibles_http.rs) | [Deadline fixture](../scripts/api-deadlines-demo.py) |
| `hearing_derived_deadlines/mod.rs` (2) | [Derived deadline boundaries](../crates/web/tests/hearing_derived_deadline_http_support/boundaries.rs) | [Derived deadline fixture](../scripts/api-hearing-derived-deadline-demo.py) |
| `hearing_results/mod.rs` (8) | [Results](../crates/web/tests/hearing_results_http.rs), [queries](../crates/web/tests/hearing_results_queries.rs) | [Result fixture](../scripts/api_hearing_results_demo.py) |
| `hearings/mod.rs` (9) | [Hearing route delegation](../crates/web/src/hearings/route_tests.rs) | [Hearing roles/foreign case fixture](../scripts/api-hearings-demo.py) |
| `participants/mod.rs` (7) | [Directory](../crates/web/tests/participant_api.rs) | [Participant role/removal fixture](../scripts/api-participant-demo.sh) |
| `precautionary/administrative_router.rs` (7) | [Administrative operations](../crates/web/tests/measure_administrative_http.rs), [current/exact records](../crates/web/tests/measure_records_http.rs) | [Precautionary fixture](../scripts/api-precautionary-demo.py) |
| `precautionary/decision_router.rs` (5) | [Decisions](../crates/web/tests/measure_decisions_http.rs) | [Precautionary fixture](../scripts/api-precautionary-demo.py) |
| `precautionary/mod.rs` (7) | [Precautionary hearings/context](../crates/web/tests/precautionary_hearings_http.rs) | [Precautionary fixture](../scripts/api-precautionary-demo.py) |
| `procedural_facts/mod.rs` (16) | [Facts](../crates/web/tests/procedural_facts_http.rs), [queries](../crates/web/tests/procedural_facts_queries.rs) | [Both fact families](../scripts/api-procedural-facts-demo.py) |
| `procedural_resources/mod.rs` (11) | [Procedural resources](../crates/web/tests/procedural_resources_http.rs) | [Roles, removed member and archive fixture](../scripts/api-procedural-resources-demo.py) |
| `resource_activities/mod.rs` (9) | [Resource associations](../crates/web/tests/resource_activities_http.rs), [reverse links](../crates/web/tests/activity_resource_links_http.rs) | [Role/removal/recovery fixture](../scripts/api-resource-activities-demo.py) |
| `resource_deadlines/mod.rs` (2) | [Resource deadlines](../crates/web/tests/resource_deadlines_http.rs) | [Contextual deadline fixture](../scripts/api-contextual-deadline-demo.py) |
| `resource_hearings/mod.rs` (5) | [Resource hearings](../crates/web/tests/resource_hearings_http.rs) | [Resource hearing fixture](../scripts/api-resource-hearings-demo.py), [access cases](../scripts/api_resource_hearings_access.py) |
| `typed_participants/mod.rs` (4) | [Proposals](../crates/web/src/typed_participants/proposal_tests.rs), [credentials](../crates/web/src/typed_participants/credential_tests.rs) | [Typed fixture](../scripts/api-typed-participant-demo.py), [native authorization](../crates/infrastructure/tests/typed_participant_authorization.rs) |
| `typed_participants/subjects.rs` (6) | [Typed route delegation](../crates/web/src/typed_participants/route_tests.rs) | [Typed fixture](../scripts/api-typed-participant-demo.py), [native authorization](../crates/infrastructure/tests/typed_participant_authorization.rs) |

The explicit document-content HEAD rejection has a direct case in
[`document_content_http.rs`](../crates/web/tests/document_content_http.rs).
Report download HEAD remains an explicit handler in
[`case_reports/mod.rs`](../crates/web/src/case_reports/mod.rs) and a B row;
no direct assertion of that handler's 405 was identified in the mapped report
tests. B alone does not close that transport assertion. The mapped report
fixture is not evidence of a positive HEAD business operation.
Neither HEAD row is included in the 223 functional operations or their latency
population.

### Accepted focused and composed evidence

The local evidence directory is `output/audit-backup-closure/`; its files are
execution receipts, not additional versioned product contracts. The accepted
focused results reusable by this matrix are:

| Scope | Recorded result | Limit of the evidence |
| --- | --- | --- |
| B and request ownership | `audit-http-green.log`: 153 web library tests passed, including catalog, cancellation, drain, origin and sink failure cases. | Synthetic handler/sink tests do not execute each family's permission workflow or durable SQL commit. |
| Typed context and legacy integrity | `audit-domain-green.log`, `audit-context-green.log`, `audit-storage-green.log`: canonical/context checks and 9 storage tests passed. | Native storage tests cover mixed history, additive migration, dump/restore and denied rewrites; composed API restoration remains separate. |
| Existing audit storage/query invariants | `audit-affected-storage-green.log`: 26 passed, one manual measurement ignored. | This later pass supersedes the failed role fixture in `audit-native-access-green.log`; that earlier filename is not a passing receipt. |
| CLI provenance/output | `audit-cli-green.log` and `audit-cli-scope-green.log`: 9 CLI tests and the explicit CLI-boundary case passed. | No current-account or network attribution is reconstructed for CLI history. |
| Query and client | `audit-application-green.log`, `audit-response-green.log`, `audit-ui-green.log`, `audit-browser-green.log`: query/identity checks, 7 HTTP response tests, 21 Node tests and 14 controlled browser cases passed. | Controlled browser evidence covers null history, V2 context, four outcomes and UUID filtering; it is distinct from real backend execution. |
| Real Qadra query | `audit-qadra-native-configured.log`: 2 real browser cases passed. | Owner pagination/context display and Litigator exclusion; this does not execute every product route, role or outcome. |
| Origin edge cases and MFA identity | `audit-proxy-final.log`: 10 focused access tests passed, including duplicate/missing XFF, absent socket and both MFA response correlations. | This adds the cases introduced after the earlier 153-test receipt; it does not attest a deployed proxy configuration. |
| HTTP collector and mixed restore export | `collector-diagnostic-restore-green.log`: 19 tests passed, including the mixed legacy/context JSONL-to-CLI boundary. | Command doubles check preservation and safe diagnostics; they do not replace native chain verification or API restoration. |
| Composed API, restart and restore | `audit-api-native-diagnostic.log`: existing role/scope workflows, Owner query, restart, exact restored history and migration/export verification passed; the migration fixture retained 4 documents and 194 audit events with identical evidence ZIP. | That invocation exited 1 at the HTTP collector, after functional restoration passed. Its functional evidence is retained separately from the subsequently corrected and accepted HTTP receipt below; it is not recorded as an exit-0 invocation. |
| Native durable access writer | `access-native-measurement.json`: 5328 durable samples, 222 route payloads, all four outcomes, three repetitions at each concurrency of 1 and 8; no failures or threshold violations. | Direct `AccessAuditService` calls exercise payloads and the durable writer, not HTTP routing or each route's permissions. |

The additional origin cases are versioned in
[the focused access tests](../crates/web/src/access_audit/tests.rs); the
mixed-context deployment export case is in
[the restore exporter test](../scripts/tests/test_deploy_restore_audit.py).
Their later receipts are listed explicitly instead of treating the earlier
153-test result as evidence for tests that did not yet exist.

No unimplemented mandatory functional pair was identified in the source/fixture
mapping. This finding does not mean that every positive/negative role scenario
has run: B uses synthetic status responses, and existing native fixtures are
family coverage rather than a 223-row per-role acceptance ledger. In particular,
assignment loss already has concrete fixtures: [case/documents](../scripts/api-case-demo.sh)
remove Paralegal and Litigator without replacing their bearer, require hidden
404s afterward, and retain assigned Client basic metadata with document denial.
[Resource workflows](../scripts/api-resource-activities-demo.py) and
[reports](../scripts/api-case-reports-demo.py) also test loss of captured access.
The accepted HTTP receipt reconciles observed requests with their durable access
records. It does not invent a new permission or classify hidden 404 as proven
denial.

### Accepted durable measurement and its finite coverage

`access-http-measurement.json` reports `accepted: true`, no violations and
`universal_route_coverage: false`. Its population is native HTTP workflows through
the pre-restore capture: **2347 successful durable samples over 207/222 pairs**,
with 1824 `access.granted`, 216 `access.denied` and 307 `access.rejected` records.
No HTTP 5xx outcome occurred in this population. `access.failed` is covered by B,
the focused failure cases and the separate native writer population; no HTTP
latency result is inferred for it.

| Durable record statistic | Microseconds |
| --- | ---: |
| Minimum | 1751 |
| p50 | 2401 |
| p95 | 27028 |
| p99 | 93135 |
| Maximum | 179653 |
| Samples at or above the exclusive 500000 limit | 0 |

The collector retains individual samples and uses nearest-rank percentiles
(`sorted[ceil(p*n)-1]`). The timer starts before `tokio::spawn_blocking` schedules
the access sink and ends after its durable commit returns. It includes sink
waiting and commit; it excludes HTTP admission, handler work-budget waiting and
handler execution. Thus 179653 microseconds is the maximum observed durable
access-record latency, not total HTTP response time. Every observed successful
sample is below 500 ms; p95 is descriptive, not the acceptance threshold.

The receipt declares revision
`ccde56c2dfa309491e1b6bc271d483a61c8a5829-working-tree`, a dev/unoptimized build
with debug information, AMD Ryzen 7 7730U, 16053874688 bytes of memory,
Linux 7.2.9-100.fc43 x86_64, and NVMe-backed btrfs for both PostgreSQL and temporary
files. PostgreSQL 16.15 had `fsync`, `full_page_writes` and `synchronous_commit`
enabled. HTTP limits were eight requests and two blocking handler operations
per server, at most two simultaneous servers, across five server generations.
These are configured limits, not a claim of a constant concurrent load. The
history grew from one event before the first access record to 6756 total events
at capture; this is not an HTTP measurement over a preseeded large history.

The separate native writer campaign began each concurrency population with
1000 history rows (376832 relation bytes). Its maxima were 7506391 ns at
concurrency 1 and 111535470 ns at concurrency 8. It covers all 222 route payloads
and all four outcomes, while excluding HTTP and the Tokio blocking pool.
These samples are not merged into the HTTP statistics. Neither campaign claims
cold PostgreSQL/OS caches or a universal worst-case guarantee.

The following list is the complete `operations_missing` from the HTTP receipt.
Every row inherits B. The linked fixtures preserve the existing functional
contract; they do not supply an unobserved HTTP timing sample.

| Unobserved functional method/path | Existing fixture mapping and limit |
| --- | --- |
| `GET /api/v1/auth/certificate-login/availability` | [Availability](../crates/web/tests/owner_login_http_cases/availability.rs): injected capability, public identity boundary and shared request admission; [composition](../crates/web/tests/owner_login_http_cases/composition_cases.rs) checks enabled/disabled routes. |
| `GET /api/v1/case-reports/litigators` | [Report transport](../crates/web/tests/case_reports_http.rs): selector projection, cursor/limit, strict query and bearer/permission denial. |
| `GET /api/v1/cases/:case/measure-decisions/:decision` | [Decision reads](../crates/web/tests/measure_decisions_http_support/reads.rs): exact G1/G2 receipt, identity, budget and integrity errors; native family fixture is [precautionary acceptance](../scripts/api-precautionary-demo.py). |
| `GET /api/v1/cases/:case/precautionary-hearings/:hearing` | [Hearing reads](../crates/web/tests/precautionary_hearings_http_support/reads.rs): current/exact selectors and substituted scope rejection; native family fixture is [precautionary acceptance](../scripts/api-precautionary-demo.py). |
| `GET /api/v1/identity/signing-certificates/:binding` | [Individual signing transport](../crates/web/tests/individual_signing_http.rs): registration/recovery/withdrawal use self-bound canonical evidence, including exact binding GET. |
| `GET /api/v1/member-invitations` | [Invitation transport](../crates/web/tests/member_invitations.rs): Owner metadata, bearer and delegated role denial; [native invitation acceptance](../crates/bin/src/serve_invitation_acceptance.rs) covers enrollment/resend/reentry. |
| `GET /api/v1/member-invitations/:id` | [Invitation transport](../crates/web/tests/member_invitations.rs): exact invitation metadata and Owner delegation; [native invitation acceptance](../crates/bin/src/serve_invitation_acceptance.rs). |
| `POST /api/v1/auth/invitations/finish` | [Invitation transport](../crates/web/tests/member_invitations.rs): public capability, no issued session and sanitized uncertainty; [native enrollment](../crates/bin/src/serve_invitation_acceptance.rs). |
| `POST /api/v1/auth/invitations/prepare` | [Invitation transport](../crates/web/tests/member_invitations.rs): public capability preparation and rejection; [native enrollment](../crates/bin/src/serve_invitation_acceptance.rs). |
| `POST /api/v1/auth/invitations/receipt` | [Invitation transport](../crates/web/tests/member_invitations.rs): exact public receipt without session issuance; [native reentry](../crates/bin/src/serve_invitation_acceptance.rs). |
| `POST /api/v1/auth/password-reset/complete` | [Completion transport](../crates/web/tests/password_reset_http_support/completion_cases.rs) and [native reset/MFA acceptance](../crates/bin/src/serve_password_reset_http_acceptance.rs). |
| `POST /api/v1/auth/password-reset/request` | [Generic request transport](../crates/web/tests/password_reset_http_support/request_cases.rs) and [native reset/MFA acceptance](../crates/bin/src/serve_password_reset_http_acceptance.rs). |
| `POST /api/v1/cases/:case_id/deadline-profiles/:id/retirement` | B covers this exact pair. [Profile transport](../crates/web/tests/deadline_profile_http.rs) tests both collection scopes and global retirement; [native restore](../crates/infrastructure/tests/deadline_profile_restore.rs) persists retirement for both global and case collections. There is no exact case-retirement HTTP sample here. |
| `POST /api/v1/member-invitations` | [Invitation transport](../crates/web/tests/member_invitations.rs): exact Owner intent and role denial; [native invitation creation](../crates/bin/src/serve_invitation_acceptance.rs). |
| `POST /api/v1/member-invitations/:id/resend` | [Invitation transport](../crates/web/tests/member_invitations.rs): exact resend intent and Owner delegation; [native resend/reentry](../crates/bin/src/serve_invitation_acceptance.rs). |

No mandatory functional row lacks a source implementation and a mapped common
boundary/policy fixture. The 15 missing HTTP observations, the absent observed
5xx population and the separate HEAD transport assertion remain explicit limits
of this accepted finite evidence; they do not create a new feature or campaign.

The existing API reproduction is `bash scripts/api-demo.sh`, with an absolute
`TT_ACCESS_AUDIT_RECEIPT`, declared `AUDIT_ACCESS_REVISION` and
`AUDIT_ACCESS_ENVIRONMENT` for
[HTTP receipt reconciliation](../scripts/api-access-audit-demo.py). The Qadra reproduction is
`TT_WEB_LIVE_SHARD=3/3 bash scripts/web-demo.sh audit-events.spec.mjs` with the
existing reviewed native dependency configuration. These are coordinator-only,
sequential commands; no new runner or feature is required by this map.

## Acceptance ledger

- [x] Map every catalog row to its current composition, policy and reproducible
      positive/negative fixtures. B covers all 223 pairs plus the two explicit
      HEAD patterns; composed API and native fixtures supply the existing
      workflow acceptance. The 15 unobserved HTTP pairs are enumerated above.
- [x] Verify existing role, membership, revocation, closure, wrong-resource and
      public identity scenarios through focused/native/composed fixtures and
      exactly one correlated access outcome for each observed admitted request.
      This is finite family coverage, not every role/resource Cartesian product.
- [x] Verify V2 metadata integrity, unchanged legacy hashes, append-only grants,
      populated migration, restart and backup/restore without invented history.
      Native mixed-history storage, export preservation and composed API
      restoration passed; the collector failure is separated from those results.
- [x] Verify direct/proxy origin rules and secret exclusion, including malformed
      headers, rejected input and unavailable account observations.
      Focused transport/context tests passed, including the final origin/MFA
      cases; this check does not claim deployment or real-person attribution.
- [x] Verify cancellation, capacity and sink failure without duplicate outcomes,
      false success or false rollback of a completed business operation. Common
      boundary, retained request/drain and failure cases passed in focused tests.
- [x] Verify Owner query/UI compatibility, finite text budgets and history with
      unavailable metadata; preserve full-principal revocation before disclosure.
      Focused query/client, real Owner/Litigator Qadra and composed Owner
      query/restore acceptance passed.
- [x] Retain the declared durable-latency populations with individual samples,
      maximum, distribution and failures: accepted HTTP coverage is 207/222
      pairs and 2347 samples, all strictly below 500 ms; separate native writer
      coverage is 222 route payloads and four outcomes. No HTTP timings are
      claimed for the 15 missing pairs or an unobserved 5xx outcome.
- [ ] Reconcile contracts, verification report, operations and manuscript with the
      reproduced delivery and verify the manuscript PDF (accepted locally: 414
      pages); complete exact-revision CI/integration gates. Keep local acceptance, integration and deployment as
      separate evidence. Items 1-7 use the recorded combined evidence above,
      not source-fixture existence alone.

## Exact source catalog

The following TSV is a versioned finite fixture, not a runtime permission engine.
`policy` references the table above. `source` is repository-relative; composition
is [api.rs](../crates/web/src/api.rs). One row is one functional method/path pair.
These are route patterns, not requests containing real identifiers or secrets.

```tsv
method	path	policy	source
GET	/api/v1/agenda	staff-agenda	crates/web/src/agenda/mod.rs
GET	/api/v1/alert-preferences	personal-alerts	crates/web/src/alerts/mod.rs
PUT	/api/v1/alert-preferences	personal-alerts	crates/web/src/alerts/mod.rs
GET	/api/v1/alerts	personal-alerts	crates/web/src/alerts/mod.rs
GET	/api/v1/alerts/:id	personal-alerts	crates/web/src/alerts/mod.rs
POST	/api/v1/alerts/:id/read	personal-alerts	crates/web/src/alerts/mod.rs
GET	/api/v1/audit/events	owner-audit	crates/web/src/audit_events/mod.rs
GET	/api/v1/audit/verify	owner-audit	crates/web/src/routes.rs
POST	/api/v1/auth/activity	session	crates/web/src/routes.rs
POST	/api/v1/auth/bootstrap	bootstrap	crates/web/src/routes.rs
GET	/api/v1/auth/certificate-bindings/:binding	owner-binding	crates/web/src/owner_certificates/mod.rs
POST	/api/v1/auth/certificate-bindings/:binding/prepare	owner-binding	crates/web/src/owner_certificates/mod.rs
POST	/api/v1/auth/certificate-bindings/:binding/register	owner-binding	crates/web/src/owner_certificates/mod.rs
POST	/api/v1/auth/certificate-bindings/:binding/withdraw	owner-binding	crates/web/src/owner_certificates/mod.rs
GET	/api/v1/auth/certificate-bindings/current	owner-binding	crates/web/src/owner_certificates/mod.rs
GET	/api/v1/auth/certificate-login/availability	certificate-login	crates/web/src/owner_login/mod.rs
POST	/api/v1/auth/certificate-login/proof	certificate-login	crates/web/src/owner_login/mod.rs
POST	/api/v1/auth/certificate-login/start	certificate-login	crates/web/src/owner_login/mod.rs
POST	/api/v1/auth/invitations/finish	invitation-recipient	crates/web/src/member_invitations/mod.rs
POST	/api/v1/auth/invitations/prepare	invitation-recipient	crates/web/src/member_invitations/mod.rs
POST	/api/v1/auth/invitations/receipt	invitation-recipient	crates/web/src/member_invitations/mod.rs
POST	/api/v1/auth/login	password-login	crates/web/src/routes.rs
POST	/api/v1/auth/logout	session	crates/web/src/routes.rs
GET	/api/v1/auth/me	session	crates/web/src/routes.rs
POST	/api/v1/auth/mfa/recovery	mfa	crates/web/src/routes.rs
POST	/api/v1/auth/mfa/totp	mfa	crates/web/src/routes.rs
POST	/api/v1/auth/password-reset/complete	password-reset	crates/web/src/password_reset/mod.rs
POST	/api/v1/auth/password-reset/request	password-reset	crates/web/src/password_reset/mod.rs
GET	/api/v1/auth/session	session	crates/web/src/routes.rs
GET	/api/v1/case-administrations	case-administration	crates/web/src/case_administration/mod.rs
GET	/api/v1/case-reports	own-reports	crates/web/src/case_reports/mod.rs
POST	/api/v1/case-reports	own-reports	crates/web/src/case_reports/mod.rs
GET	/api/v1/case-reports/:id	own-reports	crates/web/src/case_reports/mod.rs
GET	/api/v1/case-reports/:id/download	own-reports	crates/web/src/case_reports/mod.rs
POST	/api/v1/case-reports/:id/notice-read	own-reports	crates/web/src/case_reports/mod.rs
GET	/api/v1/case-reports/litigators	own-reports	crates/web/src/case_reports/mod.rs
GET	/api/v1/cases	basic-cases	crates/web/src/cases.rs
POST	/api/v1/cases	basic-cases	crates/web/src/cases.rs
GET	/api/v1/cases/:case/deadlines/:id/resource-associations	staff-case	crates/web/src/resource_activities/mod.rs
POST	/api/v1/cases/:case/documents/:document/versions/:version/individual-signing/prepare	individual-signing	crates/web/src/individual_signing_documents.rs
GET	/api/v1/cases/:case/documents/:document/versions/:version/individual-signing/receipts/:operation	individual-signing	crates/web/src/individual_signing_documents.rs
POST	/api/v1/cases/:case/documents/:document/versions/:version/individual-signing/submit	individual-signing	crates/web/src/individual_signing_documents.rs
GET	/api/v1/cases/:case/documents/:document/versions/:version/individual-signing/verification	individual-signing	crates/web/src/individual_signing_documents.rs
GET	/api/v1/cases/:case/documents/:id/versions/:version/content	documents	crates/web/src/document_content.rs
GET	/api/v1/cases/:case/hearings	staff-case	crates/web/src/hearings/mod.rs
POST	/api/v1/cases/:case/hearings	staff-case	crates/web/src/hearings/mod.rs
GET	/api/v1/cases/:case/hearings/:hearing/results	staff-case	crates/web/src/hearing_results/mod.rs
POST	/api/v1/cases/:case/hearings/:hearing/results	staff-case	crates/web/src/hearing_results/mod.rs
GET	/api/v1/cases/:case/hearings/:hearing/results/:id	staff-case	crates/web/src/hearing_results/mod.rs
PUT	/api/v1/cases/:case/hearings/:hearing/results/:id	staff-case	crates/web/src/hearing_results/mod.rs
GET	/api/v1/cases/:case/hearings/:hearing/results/:id/history	staff-case	crates/web/src/hearing_results/mod.rs
GET	/api/v1/cases/:case/hearings/:hearing/results/:id/revisions/:revision	staff-case	crates/web/src/hearing_results/mod.rs
POST	/api/v1/cases/:case/hearings/:hearing/results/:id/withdrawal	staff-case	crates/web/src/hearing_results/mod.rs
POST	/api/v1/cases/:case/hearings/:hearing/results/derived-deadline/prepare	staff-case	crates/web/src/hearing_derived_deadlines/mod.rs
POST	/api/v1/cases/:case/hearings/:hearing/results/derived-deadline/submit	staff-case	crates/web/src/hearing_derived_deadlines/mod.rs
POST	/api/v1/cases/:case/hearings/:hearing/results/prepare	staff-case	crates/web/src/hearing_results/mod.rs
GET	/api/v1/cases/:case/hearings/:id	staff-case	crates/web/src/hearings/mod.rs
PUT	/api/v1/cases/:case/hearings/:id	staff-case	crates/web/src/hearings/mod.rs
POST	/api/v1/cases/:case/hearings/:id/cancellation	staff-case	crates/web/src/hearings/mod.rs
GET	/api/v1/cases/:case/hearings/:id/history	staff-case	crates/web/src/hearings/mod.rs
GET	/api/v1/cases/:case/hearings/:id/resource-associations	staff-case	crates/web/src/resource_activities/mod.rs
GET	/api/v1/cases/:case/hearings/:id/revisions/:revision	staff-case	crates/web/src/hearings/mod.rs
GET	/api/v1/cases/:case/hearings/context	staff-case	crates/web/src/hearings/mod.rs
POST	/api/v1/cases/:case/hearings/prepare	staff-case	crates/web/src/hearings/mod.rs
GET	/api/v1/cases/:case/measure-administrative-operations	staff-case	crates/web/src/precautionary/administrative_router.rs
GET	/api/v1/cases/:case/measure-administrative-operations/:operation	staff-case	crates/web/src/precautionary/administrative_router.rs
POST	/api/v1/cases/:case/measure-administrative-operations/prepare	staff-case	crates/web/src/precautionary/administrative_router.rs
POST	/api/v1/cases/:case/measure-administrative-operations/submit	staff-case	crates/web/src/precautionary/administrative_router.rs
GET	/api/v1/cases/:case/measure-decisions	staff-case	crates/web/src/precautionary/decision_router.rs
GET	/api/v1/cases/:case/measure-decisions/:decision	staff-case	crates/web/src/precautionary/decision_router.rs
GET	/api/v1/cases/:case/measure-decisions/operations/:operation	staff-case	crates/web/src/precautionary/decision_router.rs
POST	/api/v1/cases/:case/measure-decisions/prepare	staff-case	crates/web/src/precautionary/decision_router.rs
POST	/api/v1/cases/:case/measure-decisions/submit	staff-case	crates/web/src/precautionary/decision_router.rs
GET	/api/v1/cases/:case/measures	staff-case	crates/web/src/precautionary/administrative_router.rs
GET	/api/v1/cases/:case/measures/:measure	staff-case	crates/web/src/precautionary/administrative_router.rs
GET	/api/v1/cases/:case/measures/:measure/revisions/:revision	staff-case	crates/web/src/precautionary/administrative_router.rs
GET	/api/v1/cases/:case/participants	staff-case	crates/web/src/participants/mod.rs
POST	/api/v1/cases/:case/participants	staff-case	crates/web/src/participants/mod.rs
GET	/api/v1/cases/:case/participants/:id	staff-case	crates/web/src/participants/mod.rs
PUT	/api/v1/cases/:case/participants/:id	staff-case	crates/web/src/participants/mod.rs
PUT	/api/v1/cases/:case/participants/:id/directory-status	staff-case	crates/web/src/participants/mod.rs
GET	/api/v1/cases/:case/participants/:id/history	staff-case	crates/web/src/participants/mod.rs
GET	/api/v1/cases/:case/participants/:id/revisions/:revision	staff-case	crates/web/src/participants/mod.rs
GET	/api/v1/cases/:case/participants/:id/revisions/:revision/credential	staff-case	crates/web/src/typed_participants/mod.rs
POST	/api/v1/cases/:case/participants/proposals/commit	staff-case	crates/web/src/typed_participants/mod.rs
POST	/api/v1/cases/:case/participants/proposals/prepare	staff-case	crates/web/src/typed_participants/mod.rs
POST	/api/v1/cases/:case/participants/proposals/review	staff-case	crates/web/src/typed_participants/mod.rs
GET	/api/v1/cases/:case/precautionary-context	staff-case	crates/web/src/precautionary/mod.rs
GET	/api/v1/cases/:case/precautionary-hearings	staff-case	crates/web/src/precautionary/mod.rs
GET	/api/v1/cases/:case/precautionary-hearings/:hearing	staff-case	crates/web/src/precautionary/mod.rs
GET	/api/v1/cases/:case/precautionary-hearings/:hearing/revisions/:revision	staff-case	crates/web/src/precautionary/mod.rs
GET	/api/v1/cases/:case/precautionary-hearings/operations/:operation	staff-case	crates/web/src/precautionary/mod.rs
POST	/api/v1/cases/:case/precautionary-hearings/prepare	staff-case	crates/web/src/precautionary/mod.rs
POST	/api/v1/cases/:case/precautionary-hearings/submit	staff-case	crates/web/src/precautionary/mod.rs
GET	/api/v1/cases/:case/procedural-resources	staff-case	crates/web/src/procedural_resources/mod.rs
POST	/api/v1/cases/:case/procedural-resources	staff-case	crates/web/src/procedural_resources/mod.rs
GET	/api/v1/cases/:case/procedural-resources/:id	staff-case	crates/web/src/procedural_resources/mod.rs
PUT	/api/v1/cases/:case/procedural-resources/:id	staff-case	crates/web/src/procedural_resources/mod.rs
GET	/api/v1/cases/:case/procedural-resources/:id/activities	staff-case	crates/web/src/resource_activities/mod.rs
POST	/api/v1/cases/:case/procedural-resources/:id/activities	staff-case	crates/web/src/resource_activities/mod.rs
GET	/api/v1/cases/:case/procedural-resources/:id/activities/:association	staff-case	crates/web/src/resource_activities/mod.rs
GET	/api/v1/cases/:case/procedural-resources/:id/activities/:association/history	staff-case	crates/web/src/resource_activities/mod.rs
GET	/api/v1/cases/:case/procedural-resources/:id/activities/:association/revisions/:revision	staff-case	crates/web/src/resource_activities/mod.rs
POST	/api/v1/cases/:case/procedural-resources/:id/activities/:association/unlink	staff-case	crates/web/src/resource_activities/mod.rs
POST	/api/v1/cases/:case/procedural-resources/:id/activities/deadlines/prepare	staff-case	crates/web/src/resource_deadlines/mod.rs
POST	/api/v1/cases/:case/procedural-resources/:id/activities/deadlines/submit	staff-case	crates/web/src/resource_deadlines/mod.rs
POST	/api/v1/cases/:case/procedural-resources/:id/activities/prepare	staff-case	crates/web/src/resource_activities/mod.rs
GET	/api/v1/cases/:case/procedural-resources/:id/activities/resource-hearings	staff-case	crates/web/src/resource_hearings/mod.rs
GET	/api/v1/cases/:case/procedural-resources/:id/activities/resource-hearings/:hearing	staff-case	crates/web/src/resource_hearings/mod.rs
GET	/api/v1/cases/:case/procedural-resources/:id/activities/resource-hearings/:hearing/revisions/:revision	staff-case	crates/web/src/resource_hearings/mod.rs
POST	/api/v1/cases/:case/procedural-resources/:id/activities/resource-hearings/prepare	staff-case	crates/web/src/resource_hearings/mod.rs
POST	/api/v1/cases/:case/procedural-resources/:id/activities/resource-hearings/submit	staff-case	crates/web/src/resource_hearings/mod.rs
POST	/api/v1/cases/:case/procedural-resources/:id/acts	staff-case	crates/web/src/procedural_resources/mod.rs
PUT	/api/v1/cases/:case/procedural-resources/:id/acts/:act	staff-case	crates/web/src/procedural_resources/mod.rs
POST	/api/v1/cases/:case/procedural-resources/:id/archive	staff-case	crates/web/src/procedural_resources/mod.rs
GET	/api/v1/cases/:case/procedural-resources/:id/history	staff-case	crates/web/src/procedural_resources/mod.rs
POST	/api/v1/cases/:case/procedural-resources/:id/reactivation	staff-case	crates/web/src/procedural_resources/mod.rs
GET	/api/v1/cases/:case/procedural-resources/:id/revisions/:revision	staff-case	crates/web/src/procedural_resources/mod.rs
POST	/api/v1/cases/:case/procedural-resources/prepare	staff-case	crates/web/src/procedural_resources/mod.rs
GET	/api/v1/cases/:case/resolutions	staff-case	crates/web/src/procedural_facts/mod.rs
POST	/api/v1/cases/:case/resolutions	staff-case	crates/web/src/procedural_facts/mod.rs
GET	/api/v1/cases/:case/resolutions/:resolution	staff-case	crates/web/src/procedural_facts/mod.rs
PUT	/api/v1/cases/:case/resolutions/:resolution	staff-case	crates/web/src/procedural_facts/mod.rs
GET	/api/v1/cases/:case/resolutions/:resolution/history	staff-case	crates/web/src/procedural_facts/mod.rs
GET	/api/v1/cases/:case/resolutions/:resolution/notifications	staff-case	crates/web/src/procedural_facts/mod.rs
POST	/api/v1/cases/:case/resolutions/:resolution/notifications	staff-case	crates/web/src/procedural_facts/mod.rs
GET	/api/v1/cases/:case/resolutions/:resolution/notifications/:notification	staff-case	crates/web/src/procedural_facts/mod.rs
PUT	/api/v1/cases/:case/resolutions/:resolution/notifications/:notification	staff-case	crates/web/src/procedural_facts/mod.rs
GET	/api/v1/cases/:case/resolutions/:resolution/notifications/:notification/history	staff-case	crates/web/src/procedural_facts/mod.rs
GET	/api/v1/cases/:case/resolutions/:resolution/notifications/:notification/revisions/:revision	staff-case	crates/web/src/procedural_facts/mod.rs
POST	/api/v1/cases/:case/resolutions/:resolution/notifications/:notification/withdrawal	staff-case	crates/web/src/procedural_facts/mod.rs
POST	/api/v1/cases/:case/resolutions/:resolution/notifications/prepare	staff-case	crates/web/src/procedural_facts/mod.rs
GET	/api/v1/cases/:case/resolutions/:resolution/revisions/:revision	staff-case	crates/web/src/procedural_facts/mod.rs
POST	/api/v1/cases/:case/resolutions/:resolution/withdrawal	staff-case	crates/web/src/procedural_facts/mod.rs
POST	/api/v1/cases/:case/resolutions/prepare	staff-case	crates/web/src/procedural_facts/mod.rs
GET	/api/v1/cases/:case/subjects	staff-case	crates/web/src/typed_participants/subjects.rs
GET	/api/v1/cases/:case/subjects/:id	staff-case	crates/web/src/typed_participants/subjects.rs
PUT	/api/v1/cases/:case/subjects/:id	staff-case	crates/web/src/typed_participants/subjects.rs
GET	/api/v1/cases/:case/subjects/:id/history	staff-case	crates/web/src/typed_participants/subjects.rs
POST	/api/v1/cases/:case/subjects/:id/review	staff-case	crates/web/src/typed_participants/subjects.rs
GET	/api/v1/cases/:case/subjects/:id/revisions/:revision	staff-case	crates/web/src/typed_participants/subjects.rs
GET	/api/v1/cases/:case_id/deadline-profiles	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
POST	/api/v1/cases/:case_id/deadline-profiles	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
GET	/api/v1/cases/:case_id/deadline-profiles/:id	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
PUT	/api/v1/cases/:case_id/deadline-profiles/:id	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
GET	/api/v1/cases/:case_id/deadline-profiles/:id/history	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
POST	/api/v1/cases/:case_id/deadline-profiles/:id/retirement	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
GET	/api/v1/cases/:case_id/deadline-profiles/:id/revisions/:revision	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
POST	/api/v1/cases/:case_id/deadline-profiles/prepare	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
GET	/api/v1/cases/:case_id/deadlines	staff-case	crates/web/src/deadlines/mod.rs
POST	/api/v1/cases/:case_id/deadlines	staff-case	crates/web/src/deadlines/mod.rs
GET	/api/v1/cases/:case_id/deadlines/:id	staff-case	crates/web/src/deadlines/mod.rs
PUT	/api/v1/cases/:case_id/deadlines/:id	staff-case	crates/web/src/deadlines/mod.rs
POST	/api/v1/cases/:case_id/deadlines/:id/attention	staff-case	crates/web/src/deadlines/mod.rs
GET	/api/v1/cases/:case_id/deadlines/:id/history	staff-case	crates/web/src/deadlines/mod.rs
POST	/api/v1/cases/:case_id/deadlines/:id/retirement	staff-case	crates/web/src/deadlines/mod.rs
GET	/api/v1/cases/:case_id/deadlines/:id/revisions/:revision	staff-case	crates/web/src/deadlines/mod.rs
POST	/api/v1/cases/:case_id/deadlines/prepare	staff-case	crates/web/src/deadlines/mod.rs
GET	/api/v1/cases/:case_id/deadlines/responsibles	staff-case	crates/web/src/deadlines/mod.rs
GET	/api/v1/cases/:case_id/documents	documents	crates/web/src/routes.rs
POST	/api/v1/cases/:case_id/documents	documents	crates/web/src/routes.rs
GET	/api/v1/cases/:case_id/documents/:id	documents	crates/web/src/routes.rs
GET	/api/v1/cases/:case_id/documents/:id/evidence	documents	crates/web/src/routes.rs
GET	/api/v1/cases/:case_id/documents/:id/metadata	documents	crates/web/src/routes.rs
PUT	/api/v1/cases/:case_id/documents/:id/metadata	documents	crates/web/src/routes.rs
GET	/api/v1/cases/:case_id/documents/:id/metadata/history	documents	crates/web/src/routes.rs
POST	/api/v1/cases/:case_id/documents/:id/seal	documents	crates/web/src/routes.rs
POST	/api/v1/cases/:case_id/documents/:id/verify	documents	crates/web/src/routes.rs
GET	/api/v1/cases/:case_id/documents/:id/versions	documents	crates/web/src/routes.rs
POST	/api/v1/cases/:case_id/documents/:id/versions	documents	crates/web/src/routes.rs
GET	/api/v1/cases/:case_id/documents/:id/versions/:version	documents	crates/web/src/routes.rs
GET	/api/v1/cases/:case_id/documents/:id/versions/:version/evidence	documents	crates/web/src/routes.rs
POST	/api/v1/cases/:case_id/documents/:id/versions/:version/seal	documents	crates/web/src/routes.rs
POST	/api/v1/cases/:case_id/documents/:id/versions/:version/verify	documents	crates/web/src/routes.rs
POST	/api/v1/cases/:case_id/documents/with-metadata	documents	crates/web/src/routes.rs
GET	/api/v1/cases/:case_id/members	owner-members	crates/web/src/members/mod.rs
GET	/api/v1/cases/:id	basic-cases	crates/web/src/cases.rs
GET	/api/v1/cases/:id/administration	case-administration	crates/web/src/case_administration/mod.rs
PUT	/api/v1/cases/:id/administration	case-administration	crates/web/src/case_administration/mod.rs
GET	/api/v1/cases/:id/administration/history	case-administration	crates/web/src/case_administration/mod.rs
PUT	/api/v1/cases/:id/administrative-status	case-administration	crates/web/src/case_administration/mod.rs
DELETE	/api/v1/cases/:id/members/:user_id	basic-cases	crates/web/src/cases.rs
PUT	/api/v1/cases/:id/members/:user_id	basic-cases	crates/web/src/cases.rs
GET	/api/v1/cases/:id/stage	staff-case	crates/web/src/case_stages/mod.rs
POST	/api/v1/cases/:id/stage/adoption	staff-case	crates/web/src/case_stages/mod.rs
GET	/api/v1/cases/:id/stage/history	staff-case	crates/web/src/case_stages/mod.rs
POST	/api/v1/cases/:id/stage/transitions	staff-case	crates/web/src/case_stages/mod.rs
GET	/api/v1/dashboard	dashboard	crates/web/src/dashboard.rs
GET	/api/v1/deadline-profiles	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
POST	/api/v1/deadline-profiles	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
GET	/api/v1/deadline-profiles/:id	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
PUT	/api/v1/deadline-profiles/:id	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
GET	/api/v1/deadline-profiles/:id/history	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
POST	/api/v1/deadline-profiles/:id/retirement	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
GET	/api/v1/deadline-profiles/:id/revisions/:revision	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
POST	/api/v1/deadline-profiles/prepare	deadline-profiles	crates/web/src/deadline_profiles/mod.rs
GET	/api/v1/document-integrity-incidents	owner-integrity	crates/web/src/document_integrity.rs
GET	/api/v1/document-integrity-incidents/:id	owner-integrity	crates/web/src/document_integrity.rs
GET	/api/v1/evidence/trust	session	crates/web/src/evidence_trust/mod.rs
GET	/api/v1/hearings	staff-agenda	crates/web/src/hearings/mod.rs
GET	/api/v1/identity/signing-certificates/:binding	litigator-binding	crates/web/src/individual_signing.rs
POST	/api/v1/identity/signing-certificates/:binding/prepare	litigator-binding	crates/web/src/individual_signing.rs
POST	/api/v1/identity/signing-certificates/:binding/register	litigator-binding	crates/web/src/individual_signing.rs
POST	/api/v1/identity/signing-certificates/:binding/withdraw	litigator-binding	crates/web/src/individual_signing.rs
GET	/api/v1/identity/signing-certificates/current	litigator-binding	crates/web/src/individual_signing.rs
GET	/api/v1/judicial-calendars	global-calendars	crates/web/src/judicial_calendars/mod.rs
POST	/api/v1/judicial-calendars	global-calendars	crates/web/src/judicial_calendars/mod.rs
GET	/api/v1/judicial-calendars/:id	global-calendars	crates/web/src/judicial_calendars/mod.rs
PUT	/api/v1/judicial-calendars/:id	global-calendars	crates/web/src/judicial_calendars/mod.rs
GET	/api/v1/judicial-calendars/:id/history	global-calendars	crates/web/src/judicial_calendars/mod.rs
POST	/api/v1/judicial-calendars/:id/retirement	global-calendars	crates/web/src/judicial_calendars/mod.rs
GET	/api/v1/judicial-calendars/:id/revisions/:revision	global-calendars	crates/web/src/judicial_calendars/mod.rs
GET	/api/v1/judicial-calendars/:id/revisions/:revision/days	global-calendars	crates/web/src/judicial_calendars/mod.rs
POST	/api/v1/judicial-calendars/prepare	global-calendars	crates/web/src/judicial_calendars/mod.rs
GET	/api/v1/member-invitations	owner-invitations	crates/web/src/member_invitations/mod.rs
POST	/api/v1/member-invitations	owner-invitations	crates/web/src/member_invitations/mod.rs
GET	/api/v1/member-invitations/:id	owner-invitations	crates/web/src/member_invitations/mod.rs
POST	/api/v1/member-invitations/:id/resend	owner-invitations	crates/web/src/member_invitations/mod.rs
POST	/api/v1/penal-cases	case-administration	crates/web/src/case_administration/mod.rs
GET	/api/v1/users	owner-members	crates/web/src/members/mod.rs
POST	/api/v1/users	owner-members	crates/web/src/routes.rs
GET	/api/v1/users/:id	owner-members	crates/web/src/members/mod.rs
PUT	/api/v1/users/:id/access	owner-members	crates/web/src/members/mod.rs
```

Explicit transport rejections, excluded from the 223 functional pairs:

```tsv
method	path	policy	source
HEAD	/api/v1/case-reports/:id/download	head-rejected	crates/web/src/case_reports/mod.rs
HEAD	/api/v1/cases/:case/documents/:id/versions/:version/content	head-rejected	crates/web/src/document_content.rs
```
