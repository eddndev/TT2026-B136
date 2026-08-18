# 0006. Evidence package format

## Status

Accepted. The original MSRV constraint was superseded by
`docs/adr/0013-raise-msrv-for-security-fixes.md`; the dependency-surface
rationale remains in force.

## Context

The prototype exports an evidence package: one archive holding a
document, its detached signature, its timestamp token, the signer and
issuer certificates, the revocation list, and a generated instruction
file, so that a third party can verify everything with nothing but the
openssl command line tool. The archive format must be readable with
tools every recipient already has, which in practice means ZIP.

Two ways of producing ZIP were considered:

1. Depend on the `zip` crate. At the time of this decision the workspace
   declared `rust-version = 1.78` and pinned the der 0.7 crate family for
   that reason (docs/adr/0001-msrv-and-dependency-pinning.md). The current
   `zip` release at that time (8.x) required rustc
   1.88; the newest release that accepts rustc 1.78 is `zip` 4.2.0,
   several major versions behind and therefore outside the range where
   fixes land. Its default build also pulls a large tree of
   compression and cryptography dependencies (deflate backends,
   zopfli, bzip2, lzma, AES) for capabilities this use has no need
   for, enlarging the surface that `cargo deny check` and code review
   must vouch for.

2. Write the minimal subset of the format in the repository: stored
   entries (method 0, no compression) with correct CRC-32 values, one
   local file header per entry, the central directory, and the
   end-of-central-directory record. This subset is stable since the
   original PKWARE APPNOTE, fits in one small module, and needs no
   dependency at all.

Compression is worthless here: the members are certificates and DER
structures a few kilobytes in size, and documents that are stored next
to their digests, so byte-exact fidelity matters more than size.

## Decision

Evidence packages are written by `StoredZipWriter`
(crates/infrastructure/src/archive.rs), an in-repository implementation
of the stored-entry ZIP subset behind the domain's `ArchiveWriter`
port. Entry names are restricted by the domain to a conservative ASCII
set without path separators, so an extracted archive can never write
outside its target directory. Output is deterministic: fixed DOS-epoch
timestamps and caller-defined entry order.

Correctness is guarded by tests that go through an independent
implementation: the system `unzip` binary must accept the archive
(`unzip -t`), extract every member byte for byte, and detect a flipped
byte through the stored CRC-32
(crates/infrastructure/tests/zip_archive.rs).

## Consequences

- No new dependency; `cargo deny check` keeps its current scope. The later
  MSRV increase does not by itself justify replacing the reviewed writer.
- Packages are slightly larger than compressed ones would be, which is
  irrelevant at evidence sizes.
- The writer cannot read archives and does not implement ZIP64,
  encryption, or compression. If the system ever needs to consume
  ZIP files or exceed the 4 GiB classic limits, this decision must be
  revisited rather than the module extended.
- When the workspace's minimum toolchain moves far enough that a
  maintained `zip` release becomes eligible, replacing the module is a
  contained change behind the `ArchiveWriter` port.
