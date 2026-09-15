# PDF structure fixture

`xref-stream.b64` is the base64 encoding of this repository's synthetic
`crates/infrastructure/tests/fixtures/stage-support.pdf`, rewritten by the
pinned qpdf 12.4.1 executable with `--object-streams=generate --static-id`.
It has PDF 1.5 xref and compressed object streams. Generation is a test-fixture
operation; production validation never rewrites document bytes. qpdf's
`--check` accepted the generated file without warnings.
