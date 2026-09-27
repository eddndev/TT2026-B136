# Provisioning the bounded media decoder

The general document admission adapter requires FFmpeg and ffprobe with an
explicit capability set. An arbitrary distribution binary or ffprobe-only
installation does not establish that complete decoding is available.

## Pinned source and provenance

- Version: `9.0.2`.
- Official source: <https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz>.
- SHA-256: `8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e`.
- Upstream release signing fingerprint:
  `FCF986EA15E6E293A5644F10B4322F04D67658D8`.

The source pin was established from the official release and its upstream GPG
signature. The installer verifies the fixed SHA-256 on both downloaded and
cached archives before extraction; it does not obtain a signing key or perform
an online GPG check on each installation. A fingerprint in this document alone
is not a fresh signature-verification result.

## Host prerequisites

Use Linux with Python 3.10 or newer, GCC, make, NASM, pkg-config and zlib development
headers. On Ubuntu these correspond to `python3`, `gcc`, `make`, `nasm`,
`pkg-config` and `zlib1g-dev`, together with normal C development headers/toolchain
packages such as `build-essential`. HTTPS certificate trust is required when
fetching an uncached archive. Package installation is an administrative action;
the provisioning script does not modify system package repositories.

Compilation uses one make job. The fixed configure options disable networking,
autodetection, shared FFmpeg libraries, documentation, debug information and
unselected components. They enable fd/pipe protocols, the MOV/MP3/WAV/JPEG/PNG
demuxers and the selected H.264/AAC/MP3/PCM/JPEG/PNG decoding components. The null
output path includes its encoders and filters. Static FFmpeg libraries do not
mean every system dependency is statically linked.
Build on the destination distribution or a compatible ABI baseline. A binary
verified on Ubuntu 22 may require GLIBC 2.35 and fail on AlmaLinux 9; copying
the completion marker does not establish compatibility. Always execute the
read-only verifier on each destination host.

## Provision once, outside the CI test campaign

Run with an account allowed to create the selected installation prefix and
source cache. For an administrator-owned installation, for example:

```bash
sudo python3 -B scripts/install_media_decoder.py \
  --prefix /opt/tt-media \
  --cache /var/cache/tt-media
```

The prefix must be absolute. Source and build caches are retained. Extraction
accepts only regular files/directories within the pinned package, rejects links
and escaping paths, and does not restore archive ownership or special mode bits.
The installer installs into a private staging directory and checks both binaries,
configuration and capabilities before writing its completion marker and
publishing the prefix atomically.

An existing prefix with a mismatched marker, version or capability set is
rejected intact. Use a different explicit prefix for a new pin; do not delete a
working installation or rebuild dependencies on every CI run.

## Read-only verification for CI

After administrative provisioning, the runner verifies the installed prefix:

```bash
python3 -B scripts/install_media_decoder.py \
  --verify --prefix /opt/tt-media
```

This mode requires no cache and must not create directories, acquire a lock in
the prefix parent, download sources or compile anything. It checks the exact
completion marker and both executables' current version/configuration/capability
reports. The runner needs read/execute access only. A failure blocks use of that
installation; verification must not fall back to a different system FFmpeg.

## Runtime validation is a separate boundary

Successful provisioning does not validate uploaded files. Positive/negative
format fixtures and complete decoder execution remain the adapter's acceptance
work. The worker's address-space (AS), CPU, wall-clock and output limits remain
independent of build resources and the runner's total RAM. The runtime policy and measured focal checks are recorded in
`docs/adr/0058-bounded-general-document-admission.md`; do not raise those limits
or infer complete upload safety from this provisioning check. Existing PDF/DOCX
limits remain documented in `docs/document-format-operations.md`.
