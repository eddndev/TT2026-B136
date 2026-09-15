"""Inspect, extract, and verify the authenticated qpdf release tree."""

import hashlib
import os
from pathlib import Path, PurePosixPath
import posixpath
import stat
import zipfile

ARCHIVE_NAME = ".qpdf-release.zip"


class SetupError(Exception):
    """A native setup prerequisite or authenticated artifact check failed."""


def digest_stream(stream):
    digest = hashlib.sha256()
    while chunk := stream.read(1024 * 1024):
        digest.update(chunk)
    return digest.hexdigest()


def inspect_archive(path):
    members = {}
    with zipfile.ZipFile(path) as archive:
        infos = archive.infolist()
        if not infos or len(infos) > 256 or sum(i.file_size for i in infos) > 128 * 1024 * 1024:
            raise SetupError("release archive exceeds its layout bounds")
        for info in infos:
            name = info.filename.rstrip("/")
            pieces = name.split("/")
            if (not name or "\\" in name or "\x00" in name
                    or any(p in ("", ".", "..") for p in pieces)
                    or pieces[0] not in ("bin", "lib") or name in members
                    or info.flag_bits & 1):
                raise SetupError("release archive contains an unsafe or duplicate member")
            mode = info.external_attr >> 16
            kind = "directory" if info.is_dir() else "file"
            if stat.S_ISLNK(mode):
                kind = "symlink"
            elif mode and not (stat.S_ISREG(mode) or stat.S_ISDIR(mode)):
                raise SetupError("release archive contains a special file")
            if stat.S_ISDIR(mode) != info.is_dir() and mode:
                raise SetupError("release archive directory metadata disagrees")
            member = {"kind": kind, "mode": 0o755 if mode & 0o111 else 0o644, "size": info.file_size}
            if kind == "symlink":
                if info.file_size > 4096:
                    raise SetupError("release symlink target exceeds its bound")
                target = archive.read(info).decode("utf-8", errors="strict")
                resolved = posixpath.normpath(posixpath.join(posixpath.dirname(name), target))
                if (not target or target.startswith("/") or "\\" in target or "\x00" in target
                        or resolved == ".." or resolved.startswith("../")):
                    raise SetupError("release symlink leaves the extracted tree")
                member["target"] = target
            elif kind == "file":
                with archive.open(info) as source:
                    member["digest"] = digest_stream(source)
            members[name] = member
    for name in members:
        for parent in PurePosixPath(name).parents:
            if str(parent) in members and members[str(parent)]["kind"] != "directory":
                raise SetupError("release member has a non-directory parent")
    return members


def expected_directories(members):
    directories = set()
    for name, member in members.items():
        if member["kind"] == "directory":
            directories.add(name)
        directories.update(str(p) for p in PurePosixPath(name).parents if str(p) != ".")
    return directories


def extract_archive(archive_path, root, members):
    for directory in sorted(expected_directories(members), key=lambda p: (p.count("/"), p)):
        (root / directory).mkdir(mode=0o700)
    with zipfile.ZipFile(archive_path) as archive:
        for name, member in members.items():
            if member["kind"] != "file":
                continue
            destination = root / name
            with archive.open(name) as source, destination.open("xb") as output:
                while chunk := source.read(1024 * 1024):
                    output.write(chunk)
            destination.chmod(member["mode"])
    for name, member in members.items():
        if member["kind"] == "symlink":
            (root / name).symlink_to(member["target"])
    verify_tree(root, members)


def verify_tree(root, members):
    expected = set(members) | expected_directories(members) | {ARCHIVE_NAME}
    actual = set()
    for parent, directories, files in os.walk(root, followlinks=False):
        for name in directories + files:
            actual.add((Path(parent) / name).relative_to(root).as_posix())
    if actual != expected:
        raise SetupError("cached release contains missing or unexpected paths")
    for directory in expected_directories(members):
        path = root / directory
        if path.is_symlink() or not path.is_dir():
            raise SetupError("cached release has an unsafe directory")
    for name, member in members.items():
        path = root / name
        mode = path.lstat().st_mode
        if member["kind"] == "symlink":
            if not stat.S_ISLNK(mode) or os.readlink(path) != member["target"]:
                raise SetupError("cached release symlink differs from the archive")
            try:
                resolved = path.resolve(strict=True)
            except (OSError, RuntimeError) as error:
                raise SetupError("release symlink has no contained regular file target") from error
            if not resolved.is_relative_to(root.resolve()) or not resolved.is_file():
                raise SetupError("cached release symlink leaves the tree or has no file target")
        elif member["kind"] == "file":
            if (not stat.S_ISREG(mode) or stat.S_IMODE(mode) != member["mode"]
                    or path.stat().st_size != member["size"]):
                raise SetupError("cached release file type or permissions differ")
            with path.open("rb") as source:
                if digest_stream(source) != member["digest"]:
                    raise SetupError("cached release file differs from the verified archive")
