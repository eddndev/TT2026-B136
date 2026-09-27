"""Provision a pinned minimal FFmpeg build without modifying system packages."""

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shlex
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request

VERSION = "9.0.2"
SOURCE_URL = "https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz"
SOURCE_SHA256 = "8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e"
# Provenance: official FFmpeg release signature; the fixed digest is mandatory.
SIGNING_FINGERPRINT = "FCF986EA15E6E293A5644F10B4322F04D67658D8"
MARKER = ".tt-media-decoder.json"
ARCHIVE_LIMIT = 128 * 1024 * 1024
EXTRACT_LIMIT = 512 * 1024 * 1024
DEMUXERS = ("mov", "mp3", "wav", "image_jpeg_pipe", "image_png_pipe")
DECODERS = ("h264", "aac", "mp3", "mp3float", "mjpeg", "png", "pcm_u8",
            "pcm_s16le", "pcm_s24le", "pcm_s32le")
PARSERS = ("h264", "aac", "mpegaudio", "mjpeg", "png")
FILTERS = ("null", "anull", "aformat", "format", "scale", "aresample")


def configure_args(prefix):
    prefix = Path(prefix)
    if not prefix.is_absolute() or prefix == Path("/"):
        raise ValueError("installation prefix must be an absolute non-root directory")
    return [
        f"--prefix={prefix}", "--disable-network", "--disable-autodetect",
        "--disable-everything", "--disable-doc", "--disable-debug", "--disable-shared",
        "--enable-static", "--enable-ffmpeg", "--enable-ffprobe", "--disable-ffplay",
        "--enable-zlib", "--enable-pthreads", "--enable-protocol=fd,pipe",
        "--enable-demuxer=" + ",".join(DEMUXERS),
        "--enable-decoder=" + ",".join(DECODERS),
        "--enable-parser=" + ",".join(PARSERS), "--enable-muxer=null",
        "--enable-encoder=wrapped_avframe,pcm_s16le",
        "--enable-filter=" + ",".join(FILTERS),
    ]


def manifest(prefix):
    return {"version": VERSION, "source_url": SOURCE_URL, "sha256": SOURCE_SHA256,
            "signing_fingerprint": SIGNING_FINGERPRINT,
            "configure": configure_args(prefix), "build_jobs": 1}


def verify_archive(path):
    digest = hashlib.sha256()
    size = 0
    with Path(path).open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            size += len(block)
            if size > ARCHIVE_LIMIT:
                raise RuntimeError("source archive exceeds the bounded download size")
            digest.update(block)
    if digest.hexdigest() != SOURCE_SHA256:
        raise RuntimeError("FFmpeg source SHA-256 digest mismatch")


def source_archive(cache):
    cache = Path(cache)
    cache.mkdir(parents=True, exist_ok=True)
    archive = cache / f"ffmpeg-{VERSION}.tar.xz"
    if archive.exists():
        verify_archive(archive)
        return archive
    descriptor, name = tempfile.mkstemp(prefix=".ffmpeg-download-", dir=cache)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "wb") as destination, urllib.request.urlopen(
            SOURCE_URL, timeout=60
        ) as source:
            size = 0
            while True:
                block = source.read(1024 * 1024)
                if not block:
                    break
                size += len(block)
                if size > ARCHIVE_LIMIT:
                    raise RuntimeError("source archive exceeds the bounded download size")
                destination.write(block)
        verify_archive(temporary)
        if archive.exists():
            verify_archive(archive)
        else:
            temporary.replace(archive)
        return archive
    finally:
        temporary.unlink(missing_ok=True)


def extract_source(archive, build_root):
    verify_archive(archive)
    build_root = Path(build_root)
    build_root.mkdir(parents=True, exist_ok=True)
    package = f"ffmpeg-{VERSION}"
    source = build_root / package
    receipt = build_root / ".tt-source-sha256"
    if source.exists():
        if source.is_symlink() or not receipt.is_file() or receipt.read_text().strip() != SOURCE_SHA256:
            raise RuntimeError("existing source cache has no matching extraction receipt")
        return source
    with tempfile.TemporaryDirectory(prefix=".ffmpeg-extract-", dir=build_root) as staging:
        staging = Path(staging)
        with tarfile.open(archive, "r:xz") as stream:
            members = stream.getmembers()
            total = 0
            if len(members) > 100000:
                raise RuntimeError("source archive exceeds the member limit")
            for member in members:
                name = PurePosixPath(member.name)
                if (name.is_absolute() or ".." in name.parts or not name.parts
                        or name.parts[0] != package or "\\" in member.name):
                    raise RuntimeError("source archive path escapes its package")
                # Extract only regular files and directories: no links, devices,
                # owner restoration or special mode bits, including on Python 3.10.
                if not (member.isfile() or member.isdir()):
                    raise RuntimeError("source archive links and special files are forbidden")
                total += member.size
                if member.size < 0 or total > EXTRACT_LIMIT:
                    raise RuntimeError("source archive exceeds the extraction size limit")
            for member in members:
                destination = staging.joinpath(*PurePosixPath(member.name).parts)
                if member.isdir():
                    destination.mkdir(parents=True, exist_ok=True)
                    continue
                destination.parent.mkdir(parents=True, exist_ok=True)
                with stream.extractfile(member) as original, destination.open("xb") as output:
                    shutil.copyfileobj(original, output)
                destination.chmod(0o755 if member.mode & 0o111 else 0o644)
        (staging / package).rename(source)
        receipt.write_text(SOURCE_SHA256 + "\n")
    return source


def capability_names(output):
    names = set()
    for line in output.splitlines():
        fields = line.split()
        if len(fields) >= 2 and re.fullmatch(r"[A-Z.]{1,6}", fields[0]):
            if fields[1] != "=":
                names.update(fields[1].split(","))
    return names


def protocols(output):
    result = set()
    active = False
    for line in output.splitlines():
        value = line.strip()
        if value in {"Input:", "Output:"}:
            active = True
        elif active and value:
            if not re.fullmatch(r"[a-z0-9_]+", value):
                raise RuntimeError("unexpected media protocol listing")
            result.add(value)
    return result


def verify_binaries(prefix, configured_prefix=None):
    prefix = Path(prefix)
    expected = configure_args(configured_prefix or prefix)
    for program in ("ffmpeg", "ffprobe"):
        binary = prefix / "bin" / program
        if not binary.is_file() or binary.is_symlink() or not os.access(binary, os.X_OK):
            raise RuntimeError(f"missing regular executable: {program}")

        def capture(option):
            output = subprocess.check_output(
                [str(binary), "-hide_banner", option], text=True, stderr=subprocess.STDOUT,
                timeout=30,
            )
            # The pinned ffmpeg help callback prints a final success footer even
            # at error log level. Preserve every preceding configuration token.
            footer = "\nExiting with exit code 0"
            trimmed = output.rstrip("\n")
            if program == "ffmpeg" and trimmed.endswith(footer):
                return trimmed[:-len(footer)] + "\n"
            return output

        version = capture("-version").splitlines()
        if not version or not re.match(rf"^{program} version {re.escape(VERSION)}(?:\s|$)", version[0]):
            raise RuntimeError(f"incorrect {program} version")
        configuration = capture("-buildconf")
        if "configuration:" not in configuration:
            raise RuntimeError("media build configuration is unavailable")
        configuration = configuration.split("configuration:", 1)[1]
        actual = shlex.split(configuration)
        if actual != expected:
            raise RuntimeError("media build configuration differs from the pinned policy")
        if protocols(capture("-protocols")) != {"fd", "pipe"}:
            raise RuntimeError("media protocol capability escapes the local descriptor policy")
        requirements = {
            "-demuxers": {"mov", "mp3", "wav", "jpeg_pipe", "png_pipe"},
            "-decoders": set(DECODERS), "-muxers": {"null"},
            "-encoders": {"wrapped_avframe", "pcm_s16le"}, "-filters": set(FILTERS),
        }
        for option, required in requirements.items():
            if not required <= capability_names(capture(option)):
                raise RuntimeError(f"required media capability is absent: {option}")


def verify_install(prefix):
    prefix = Path(prefix).expanduser()
    configure_args(prefix)
    prefix = prefix.resolve()
    marker = prefix / MARKER
    if not marker.is_file() or marker.is_symlink():
        raise RuntimeError("existing installation has no matching completion marker")
    try:
        recorded = json.loads(marker.read_text())
    except (OSError, ValueError) as error:
        raise RuntimeError("completion marker cannot be verified") from error
    if recorded != manifest(prefix):
        raise RuntimeError("existing installation has no matching completion marker")
    verify_binaries(prefix)
    return prefix


def install(prefix, cache):
    prefix = Path(prefix).expanduser()
    configure_args(prefix)
    prefix = prefix.resolve()
    cache = Path(cache).expanduser().resolve()
    prefix.parent.mkdir(parents=True, exist_ok=True)
    cache.mkdir(parents=True, exist_ok=True)
    with (prefix.parent / f".{prefix.name}.tt-media.lock").open("a+b") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if prefix.exists():
            return verify_install(prefix)
        archive = source_archive(cache)
        key = hashlib.sha256(json.dumps(manifest(prefix), sort_keys=True).encode()).hexdigest()[:20]
        source = extract_source(archive, cache / f"build-{VERSION}-{key}")
        environment = dict(os.environ)
        for name in ("MAKEFLAGS", "MFLAGS", "CFLAGS", "CPPFLAGS", "CXXFLAGS", "LDFLAGS"):
            environment.pop(name, None)
        subprocess.run([str(source / "configure"), *configure_args(prefix)],
                       cwd=source, env=environment, check=True)
        subprocess.run(["make", "-j1"], cwd=source, env=environment, check=True)
        with tempfile.TemporaryDirectory(prefix=f".{prefix.name}.install-", dir=prefix.parent) as staging:
            subprocess.run(["make", "-j1", f"DESTDIR={staging}", "install"],
                           cwd=source, env=environment, check=True)
            installed = Path(staging) / prefix.relative_to("/")
            verify_binaries(installed, configured_prefix=prefix)
            (installed / MARKER).write_text(json.dumps(manifest(prefix), indent=2, sort_keys=True) + "\n")
            if prefix.exists():
                raise RuntimeError("installation prefix appeared before atomic publication")
            installed.rename(prefix)
        return prefix


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prefix", required=True, type=Path)
    parser.add_argument("--cache", type=Path)
    parser.add_argument("--verify", action="store_true", help="verify an existing installation without writes or a cache")
    args = parser.parse_args()
    if args.verify:
        prefix = verify_install(args.prefix)
    else:
        if args.cache is None:
            parser.error("--cache is required when installing")
        prefix = install(args.prefix, args.cache)
    print(prefix / "bin" / "ffmpeg")
    print(prefix / "bin" / "ffprobe")


if __name__ == "__main__":
    main()
