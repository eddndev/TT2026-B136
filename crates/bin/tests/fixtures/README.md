# Worker budget fixture

`expanded-budget-docx.b64` encodes an owned synthetic ZIP32 DOCX package.
It contains content types, the package relationship, a minimal Word document
and an opaque `word/media/zeros.bin` part of exactly 34 MiB of zero bytes.
The first three parts have 784 bytes in total. The media part is declared as
`application/octet-stream`; the fixture exercises structural admission and
expansion limits, without making any claim about rendered appearance.

Python 3 `zipfile.ZipFile` generated all four entries with Deflate level 9,
Unix mode `0100600` and the fixed timestamp `2026-09-15 00:00:00`. The decoded
archive SHA-256 is `8f83449a9452795522be8be484aa23f6c352f17cc0ceb9269772206f6c7da274`.
The archive stays far below the 16 MiB compressed input limit, and a single
package stays below the 64 MiB expanded limit. Two copies exceed the shared
expanded budget, so the native worker test detects an accidental per-file reset.
