# 0017: Verified Times New Roman installation for report builds

## Status

Accepted.

## Context

The report selects Times New Roman through `fontspec`. The Ubuntu
`ttf-mscorefonts-installer` package downloads multiple font families through
SourceForge mirrors. An HTTP mirror timeout can leave Times New Roman absent
and fail the document workflow before LaTeX starts. Installing unrelated font
families expands this failure surface.

The original `times32.exe` archive contains the four required styles. Its
SHA-256 is recorded in the
[Debian msttcorefonts 3.8.1 manifest](https://sources.debian.org/src/msttcorefonts/3.8.1/cabfiles.sha256sums/),
and the unchanged archive is distributed by the
[Corefonts project](https://sourceforge.net/projects/corefonts/files/the%20fonts/final/times32.exe/).

## Decision

The document workflow downloads that archive over HTTPS with bounded connection
time, transfer time and retries. It verifies SHA-256
`db56595ec6ef5d3de5c24994f001f03b2a13e37cee27bc25c58f6f43e8f807ab`
before using `cabextract` to extract only the four TTF files. It does not execute
the Windows installer. Fontconfig must resolve the exact Times New Roman
family before compilation proceeds.

The workflow uses the original distribution for its build environment and does
not redistribute a modified installer or add font binaries to the repository.
The original Corefonts license remains applicable. Source and font verification
failures stop the build; the workflow does not silently substitute another
font family.

## Consequences

Builds download one font archive instead of the entire Corefonts collection.
The source still depends on SourceForge availability, but failure has a bounded
duration and archive corruption or substitution fails before extraction.
Changing the archive requires reviewing and updating the expected digest.

Local verification extracted all four styles and matched their hashes to the
Times New Roman files used for the report's local rendering. The repository
continues to store document sources only; generated PDFs are workflow and
release artifacts.
