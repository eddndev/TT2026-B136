# ADR 0078: Separate CA operator and recoverable certificate operations

## Status

Accepted and implemented locally. The [CA delivery state](../ca/README.md)
separates implementation, verification, service adoption and human acceptance.
The [HTTP contract](../ca/api.md) and [operator manual](../ca/operations.md)
describe the current boundaries. This decision does not establish installation
of the portal or acceptance of a real Owner certificate login.

## Context

The existing internal CA has a root, private key, OpenSSL index, serial archive,
CRL counter and TSA certificate. The office application consumes published
credential trust and has its own Owner accounts and certificate bindings.
An office Owner is a consumer of that authority, not its administrator.

CRL publication already has recoverable maintenance under
[ADR 0063](0063-recoverable-crl-maintenance.md). It stops the office API and web
before backing up and publishing trust. An administration portal that depended
on those services would lose its own access and recovery path during the work
it initiated. Adding a portal must preserve that maintenance protocol and the
existing root rather than introducing another authority or publication engine.

Issuance and local revocation change several OpenSSL files. Publication also
crosses the filesystem, PostgreSQL and service-manager boundaries. A successful
mutation can outlive its HTTP connection. A fresh request after a lost response
could otherwise issue another certificate or create another CRL revision.

## Decision

### Separate operator authority from office roles

Compose `qadra-ca` as a separate binary with its own HTTP routes and private
configuration. Admit exactly the configured non-nil UUID and normalized email,
after password and TOTP or a recovery code. Revalidate the persisted active
account and session on protected operations. The public operator identity has
only UUID and email; office role grants do not authorize CA operations.

Reuse the identity service and cryptographic adapters with separate
`qadra_ca.users` and `qadra_ca.audit_events` tables and the Redis `ca-identity`
namespace. The stored `Client` value is an internal compatibility format. It
does not grant an office capability, and an Owner value is not admitted as a CA
operator. The CA router composes no office bootstrap, member, case, document,
certificate-login or password-reset routes.

The PostgreSQL boundary checks the exact database, current/session role,
`qadra_ca,pg_temp` search path, schema ownership, table structure and limited
runtime grants. Runtime credentials cannot insert an operator. They permit the
existing recovery-code consumption and CA audit operations, without inheriting
office access. Sharing database and Redis services does not merge identities.

Initial enrollment is a local administrative command, not a public HTTP
bootstrap. `prepare_ca_operator` creates the new credential without persistence.
The command reads the password, repetition and TOTP privately; it writes a new
owner-only delivery file with the generated UUID, TOTP material and recovery
codes. It synchronizes and rereads the exact file before inserting anything.
After TOTP confirmation it checks the delivery again, then uses the explicitly
configured database owner to insert the initial account and its audit event in
one transaction. The administrative and service configuration files are distinct.

The command does not provision schemas or roles, replace an account, reset MFA,
or print enrollment secrets. A failed or uncertain attempt preserves its file.
The file and SQL are not one transaction: a lost commit response requires account
and audit reconciliation, while an existing account closes initial enrollment.

### Keep holder keys external and preserve exact purposes

The portal accepts only bounded public CSR material. The holder creates and
retains the private key outside the CA. An operator records the reviewed holder,
reference and criterion and confirms the requested purpose and adopted root.
CSR proof of possession and root validation are separate checks; the configured
root fingerprint fixes the issuing authority. A CSR does not assert that link.

Issue only the existing `partner` and `document-signing` profiles. Verify the
issued public key, root, signature and exact profile before delivering the
certificate receipt. Issuance neither creates an office account nor registers
its certificate binding. Binding and external proof of possession remain their
existing workflows; internal certificates do not establish civil identity.

Inventory also inspects the existing TSA profile as `time-stamping`, because
its serial belongs to the same CA index and public archive. This is read-only
inventory and download support. It does not add TSA issuance, portal revocation,
Owner admission or document-signing authority. Historical inspection reports
signed dates; it does not by itself assert current validity or revocation status.

### Bind recoverable operations to fixed inputs and public evidence

The HTTP application derives the actor from the admitted CA session. The native
worker receives that actor and a closed command vocabulary, not bearer tokens,
passwords, shell text or caller-selected executable paths. Its binaries, scripts,
deployment directory and adopted root are supplied by private configuration.
Input, output and native execution have explicit bounds.

Persist an operation UUID and its original actor and parameters before mutation.
Issuance records CSR and public-key digests, the root and original OpenSSL state.
Recovery either proceeds from that exact unchanged state or identifies the
already issued certificate by its reserved serial and public evidence. It does
not infer success from process exit alone or issue under another operation ID.

Validate the existing issuer subject policy before reserving that state: CN is
required and each C/O attribute must match an attribute of the adopted root,
including its ASN.1 type and value. An authenticated CSR with an incompatible
subject is distinct from malformed material or a failed inspector process.
For a previously saved request, a proven subject-policy mismatch can produce a
terminal rejection only after matching its pinned CSR, key and root facts and
proving the complete original OpenSSL snapshot and absent leaf paths. Preserve
the original journal, actor and request; store an explicit rejection receipt,
never a successful issuance receipt. Repeated recovery returns that rejection.
Shared guards and history recognize this terminal state. Technical failures or
any changed material retain the pending guard and require investigation.

Local revocation binds the exact serial, certificate fingerprint, reason and
root to its journal. Its receipt confirms only the index change and remains
pending publication. Inventory presents index status, certificate dates and
membership in Qadra's persisted CRL separately. A local file is not proof that
Qadra has adopted the revocation.

Publication records the same portal operation UUID, expected trust revision,
root and maintenance acknowledgement. It delegates to the existing CRL runtime:
shared deployment lock, exact baseline, prepared candidate, stopped office
ingress, verified backup, durable marker, audited SQL publication and exact
successor reconciliation. The same accepted release restarts after concordance.
The portal does not provide another renewal engine or automatic schedule.

Journals live in `data/ca/operations`, `data/ca/revocations` and
`data/ca/publications`; CRL maintenance retains its own linked record. The shared
mutation guard excludes competing issuance, revocation, publication, deployment
and restoration while recovery is pending. Native signing retains the inherited
deployment lock. Reentry preserves the original actor and parameters. A partial
or unrelated state blocks progress rather than silently repairing the index,
replacing the root, decreasing counters or undoing published trust.

### Keep CA access available during office CRL maintenance

Use separate CA API and web user units and a loopback proxy. Their rendered
configuration pins the accepted release's executable, static assets and restore
guard. The CA API depends on the existing PostgreSQL and Redis units; CA web
depends on the CA API. Neither depends on the office API or web units stopped
by CRL maintenance. Restoration keeps its separate admission boundary.

The renderer only produces configuration. It does not generate credentials,
install databases, create a root, install units, configure public DNS or prove
manifest acceptance. The administrator adopts the reviewed release and private
configuration through the existing deployment procedure.

## Consequences

The portal can retain its operator session and recovery interface while office
ingress is stopped. Its database and session authorities remain separate from
office roles, although it shares the host, deployment user and existing services.
This is not isolation from the Unix deployment account or host administrator.

The operation journals support explicit recovery across non-transactional
boundaries. They are not a distributed transaction or an external audit anchor.
A timeout, generic HTTP failure or lost connection does not prove rollback.
The client must consult or resume the original operation before a new mutation.
If the original state or exact result cannot be established, availability yields
to preservation of the existing CA and trust history.

Backups and restoration must preserve CA credentials, audit, private setup,
root/TSA material and operation records under their established scopes. Restored
session capabilities require their existing invalidation procedure; a restored
account does not authorize revival of a prior bearer. Concrete restoration,
deployment and user-acceptance evidence remain distinct from this decision.

No self-service enrollment, general invitations, SSO, arbitrary execution, new
certificate purposes, root replacement, OCSP or automatic CRL renewal is added.
