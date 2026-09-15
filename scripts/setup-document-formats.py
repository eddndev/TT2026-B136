#!/usr/bin/env python3
"""Provision the pinned Linux x86_64 qpdf library in a user-owned cache."""

import ctypes
import hashlib
import os
from pathlib import Path
import platform
import shutil
import stat
import sys
import tempfile
import urllib.request
import uuid
import zipfile

from document_format_archive import (
    ARCHIVE_NAME, SetupError, digest_stream, extract_archive, inspect_archive, verify_tree,
)

VERSION = "12.4.1"
DIGEST = "db9122e88ec00c76ac6a14e09ffb92406db1773d47b968911ff6e69f28c09bf9"
URL = f"https://github.com/qpdf/qpdf/releases/download/v{VERSION}/qpdf-{VERSION}-bin-linux-x86_64.zip"
LIBRARY = Path("lib/libqpdf.so.30.4.1")


def log(message):
    print(f"document-formats: {message}", file=sys.stderr)


def verify_native(path):
    try:
        library = ctypes.CDLL(str(path))
        version = library.qpdf_get_qpdf_version
        version.argtypes = []
        version.restype = ctypes.c_char_p
        if version() != VERSION.encode("ascii"):
            raise SetupError("native qpdf version differs from the pinned release")
    except (OSError, AttributeError) as error:
        raise SetupError(f"cannot load qpdf {VERSION} and its native dependencies: {error}") from error


def authenticated_archive(path):
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 64 * 1024 * 1024:
        return False
    with path.open("rb") as source:
        return digest_stream(source) == DIGEST


def cached_tree(root):
    try:
        if root.is_symlink() or not root.is_dir():
            return False
        archive = root / ARCHIVE_NAME
        if not authenticated_archive(archive):
            return False
        verify_tree(root, inspect_archive(archive))
        return True
    except (OSError, ValueError, SetupError, zipfile.BadZipFile):
        return False


def copy_release(source, destination):
    if source is None:
        log(f"downloading official qpdf {VERSION} Linux x86_64 release")
        request = urllib.request.Request(URL, headers={"User-Agent": "TT-native-format-setup"})
        stream = urllib.request.urlopen(request, timeout=60)
    else:
        log("verifying supplied release archive")
        stream = source.open("rb")
    digest = hashlib.sha256()
    total = 0
    with stream, destination.open("xb") as output:
        while chunk := stream.read(1024 * 1024):
            total += len(chunk)
            if total > 64 * 1024 * 1024:
                raise SetupError("download exceeds the release archive size bound")
            digest.update(chunk)
            output.write(chunk)
    if digest.hexdigest() != DIGEST:
        raise SetupError("release SHA-256 does not match the pinned official archive")


def install(cache, archive=None):
    if platform.system() != "Linux" or platform.machine() != "x86_64":
        raise SetupError("native document formats currently require Linux x86_64")
    import fcntl

    cache = Path(cache).expanduser().absolute()
    if cache.is_symlink():
        raise SetupError("native cache root must be a directory, not a symlink")
    cache.mkdir(mode=0o700, parents=True, exist_ok=True)
    metadata = cache.stat()
    if metadata.st_uid != os.getuid() or stat.S_IMODE(metadata.st_mode) & 0o022:
        raise SetupError("native cache must be owned by this user and not writable by other users")
    cache = cache.resolve()
    destination = cache / f"qpdf-{VERSION}-linux-x86_64-{DIGEST[:12]}"
    lock_fd = os.open(cache / ".setup.lock", os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    with os.fdopen(lock_fd, "r+") as lock:
        lock_metadata = os.fstat(lock.fileno())
        if (not stat.S_ISREG(lock_metadata.st_mode) or lock_metadata.st_uid != os.getuid()
                or stat.S_IMODE(lock_metadata.st_mode) & 0o022):
            raise SetupError("native cache lock is not a private regular file")
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        if cached_tree(destination):
            verify_native(destination / LIBRARY)
            log("reusing verified qpdf cache")
            return destination / LIBRARY
        if os.path.lexists(destination):
            log("cached qpdf tree failed verification; preparing a replacement")
        source = Path(archive).expanduser().absolute() if archive is not None else None
        if source is None and not destination.is_symlink():
            cached_archive = destination / ARCHIVE_NAME
            if authenticated_archive(cached_archive):
                source = cached_archive
        staging = Path(tempfile.mkdtemp(prefix=".staging-", dir=cache))
        try:
            release = staging / ARCHIVE_NAME
            copy_release(source, release)
            members = inspect_archive(release)
            if LIBRARY.as_posix() not in members:
                raise SetupError("verified release does not contain the required qpdf library")
            extract_archive(release, staging, members)
            verify_native(staging / LIBRARY)
            quarantine = None
            if os.path.lexists(destination):
                quarantine = cache / f".quarantine-{uuid.uuid4().hex}"
                destination.rename(quarantine)
            try:
                staging.rename(destination)
            except OSError:
                if quarantine is not None:
                    quarantine.rename(destination)
                raise
            log("installed verified qpdf library and companion dependencies")
            return destination / LIBRARY
        finally:
            if staging.exists():
                shutil.rmtree(staging)


def main():
    try:
        if len(sys.argv) != 1:
            raise SetupError("configure TT_DOCUMENT_FORMATS_CACHE and optional TT_QPDF_ARCHIVE through the environment")
        default = Path(os.environ.get("XDG_CACHE_HOME", str(Path.home() / ".cache"))) / "tt" / "document-formats"
        cache = Path(os.environ.get("TT_DOCUMENT_FORMATS_CACHE", str(default)))
        archive = os.environ.get("TT_QPDF_ARCHIVE") or None
        print(install(cache, archive))
        return 0
    except (SetupError, OSError, ValueError, zipfile.BadZipFile) as error:
        log(f"setup failed: {error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
