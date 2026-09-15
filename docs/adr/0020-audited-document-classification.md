# Audited document classification independent of content versions

## Status

Accepted.

## Context

`docs/adr/0019-immutable-document-versions.md` preserves encrypted snapshots and
their captured evidence. Finding documents also requires organizational values
that can change without replacing an uploaded file or invalidating its evidence.
Embedding those values into a historical snapshot would confuse current filing
decisions with the content that was originally signed and timestamped.

Classification must obey the same case authorization and transactional audit
boundary as document operations. A classified upload must not leave an uploaded
file without its requested classification when a second request fails.

## Decision

### Values and revision identity

Each document series has optional manual `document_type` and `classification`
fields and individual `tags`. They organize a document; they neither grant access
nor claim a legal category or an automatically inferred classification. They are
stored in clear text, like the existing document name, and require document read
permission within the authorized case.

The domain rejects control characters before trimming Unicode whitespace. Type
and classification accept at most 80 Unicode scalar values after trimming; an
empty optional value becomes absent. A request may supply at most 20 tags before
deduplication. Each tag must contain 1 to 40 scalar values after trimming. Tags
are deduplicated by exact equality and sorted by UTF-8 bytes. Case, accents,
Unicode normalization forms, commas and interior spaces remain significant.

Metadata revision zero means that no classification decision has been stored.
Persistent revisions start at one and increase by one up to `u32::MAX`. Each
accepted replacement creates a revision, including an empty replacement or one
whose values equal its predecessor. Clearing values does not return the document
to revision zero. An expected revision prevents a stale replacement from
overwriting a concurrent decision.

`DocumentMetadata` defines canonical bytes without implementing a hash algorithm:

1. ASCII prefix `DMETA1`.
2. Type and classification, each encoded as byte zero for absence, or byte one
   followed by its UTF-8 byte length as a four-byte big-endian integer and value.
3. Tag count as a four-byte big-endian integer, then each canonical tag encoded
   as its four-byte UTF-8 length and value.

The largest valid canonical value contains 3940 bytes. SHA-256 identifies these
organizational values; the document UUID and metadata revision identify the
decision in its audit resource. Actor and time are stored separately. Equal
digests do not imply equal documents, authors or decision times.

The empty value encodes as `444d45544131000000000000`, whose SHA-256 is
`adbad13daff0fc70b3309e1e58a20aecadc00077dac6f2a04349c4001e8443bb`.
Domain, application and PostgreSQL tests compare canonical vectors, including
Unicode and a comma within one tag. PostgreSQL uses its native
[`sha256(bytea)` function](https://www.postgresql.org/docs/18/functions-binarystring.html)
for an additional digest constraint; no extension or privileged hash function is
required.

Project function and table references within these SQL bodies are qualified with
the installation schema. They therefore keep working during `pg_restore` with
an empty search path, while preserving validation and sequence checks.

Migration and runtime startup require a UTF-8 database. SQL validates Unicode
scalar limits and controls under that encoding and compares tag order using
`convert_to(value, 'UTF8')` bytes, independently of the database collation.

### Persistence and authorization

`document_metadata_revisions` references the immutable document series. Its key
combines document identity and metadata revision. Each row captures values,
digest, canonical UTC timestamp, user UUID and the user's email at that moment.
Reading history does not join the user's current email to reconstruct authorship.
An unclassified document has no invented actor, timestamp or historical row.

Insertion acquires the shared audit lock before checking the current revision.
Replacement revalidates the active actor, role, case membership and document/case
association inside the transaction before testing the expected revision. It
inserts the next immutable row and `document.metadata_changed` together. Its
resource contains case UUID, document UUID, metadata revision and digest; it does
not copy free-form tags into the audit log.

Owner, Litigator and Paralegal may classify within their existing document scope;
Client remains denied. Reads and history require `ReadDocument`, confirm current
authority and commit their audit event before returning values, including empty
results. Conflicts cannot reveal a foreign document's current revision.

The runtime may select and insert revisions but cannot update, delete, truncate
or own them. Schema checks and triggers protect canonical values, digests,
references and contiguous revisions. Startup verifies the required schema,
privileges and metadata inventory. Decoding rejects inconsistent persisted values
instead of silently repairing their normalization or digest.

### Content and classification have separate histories

Classification does not modify `DocumentRecord`, its encrypted vault, AAD,
signature, timestamp, certificates, CRL or exported evidence ZIP. Adding a content
version preserves current classification. Sealing, verifying or exporting an
exact version does not compare a metadata revision, and classification does not
compare a content version. The two operations can succeed independently.

`DocumentOverview` combines a content summary with `current_metadata` for listing,
current detail and uploads. Insert and append return the overview from their
transaction, avoiding a read that could fail after a successful commit. Exact
version detail, content history and sealing keep their content-only projection.
Verification reports and evidence archives retain their existing contracts.

Document queries choose the latest content and classification revisions before
applying filters and pagination. Type, classification and one individual tag use
exact equality, combined by AND with the existing name and sealed-state filters.
A value removed in a later metadata revision cannot match a current query.

### Atomic initial upload and browser behavior

`POST /api/v1/cases/{case_id}/documents/with-metadata` accepts one `file` part and
one JSON `metadata` part, in either order. `X-Document-Name` supplies the validated
archive name; the multipart filename is not authoritative. The existing binary
upload remains available and creates an unclassified document.

The adapter bounds the file to 16 MiB, JSON to 8 KiB and multipart body to
16 MiB plus 32 KiB. It first buffers the entire bounded request, including any
bytes after the closing multipart boundary, without trusting Content-Length.
Transport failure cannot commit even after complete parts have arrived. It then
checks each part's size and rejects missing, repeated, unknown or incomplete
parts. Every part and
organizational value must validate before cryptographic preparation or writes.
Root, content version one, metadata revision one and both success events commit
in one transaction. A failure of either event or the commit rolls back all rows.

Qadra sends a single multipart upload, including when its optional fields are
empty. It displays current classification independently of the selected content
version. Individual tag controls preserve commas and Unicode. A replacement
conflict keeps the draft, shows current values on explicit refresh and requires
the user to choose that revision as a new base before submitting again. Failed
or delayed responses cannot repopulate a document after a scope change or denial.
An uncertain upload response does not trigger an automatic duplicate upload.

## Consequences

- Classification changes gain independent, attributable history without changing
  historical file evidence or the Qadra design system.
- Migration is additive and does not fabricate classifications for existing or
  imported documents. Reconciliation continues to compare original legacy
  snapshots and the preserved audit prefix.
- Backup and restoration include classification revisions and their captured
  provenance, as well as all encrypted content versions and audit records.
- Metadata fields and historical actor emails are visible to authorized document
  readers. They are not encrypted file content or individual signing credentials.
- The database validates internal consistency; an administrative deletion of all
  metadata history or restoration of an internally coherent old database still
  requires independent continuity evidence to distinguish it from current state.
  The external anchoring limitation in `docs/adr/0007-audit-chain-anchoring.md`
  remains applicable.
