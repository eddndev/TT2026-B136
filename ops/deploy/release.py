"""Serialize verified release activation and recover the last healthy release."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import sys
import tempfile

from bundle import digest, extract_bundle, validate_version


def metadata(target):
    return json.loads((target / "release.json").read_text())


def linked(root, name):
    path = root / name
    if not path.is_symlink():
        if path.exists():
            raise ValueError(f"{name} must be a managed symlink")
        return None
    target = path.resolve(strict=True)
    if target.parent != root / "releases":
        raise ValueError("release link escapes release directory")
    return target


def point(root, name, target):
    pending = root / f".{name}.next"
    pending.unlink(missing_ok=True)
    pending.symlink_to(target, target_is_directory=True)
    pending.replace(root / name)


def activate(root, target, runtime, allow_older=False):
    previous = linked(root, "current")
    info = metadata(target)
    schema_file = root / "config/schema"
    if schema_file.exists() and schema_file.read_text().strip() != info["schema"]:
        raise ValueError("schema differs: explicit database maintenance is required")
    if previous:
        before = metadata(previous)
        current_version = tuple(map(int, validate_version(before["version"])[1:].split(".")))
        next_version = tuple(map(int, validate_version(info["version"])[1:].split(".")))
        if not allow_older and next_version < current_version:
            raise ValueError("older tag cannot automatically replace a newer release")
        if next_version == current_version and info["commit"] != before["commit"]:
            raise ValueError("an existing version cannot identify another commit")
    try:
        runtime.stop()
        runtime.backup()
        runtime.initialize(target)
        point(root, "current", target)
        runtime.start(target)
        if previous and previous != target:
            point(root, "previous", previous)
    except BaseException:
        runtime.stop()
        if previous:
            point(root, "current", previous)
            try:
                runtime.start(previous)
            except BaseException as recovery:
                runtime.stop()
                raise RuntimeError("recovery failed; ingress remains stopped; inspect journal") from recovery
        else:
            (root / "current").unlink(missing_ok=True)
        raise


def rollback(root, runtime):
    target = linked(root, "previous")
    if target is None:
        raise ValueError("no previous healthy release")
    activate(root, target, runtime, allow_older=True)


def admit(root, archive, checksum, version, commit):
    validate_version(version)
    if not re.fullmatch(r"[0-9a-f]{64}", checksum) or digest(archive) != checksum:
        raise ValueError("archive SHA-256 mismatch")
    if archive.resolve().parent != root / "incoming" or archive.is_symlink():
        raise ValueError("archive must be a regular file in incoming")
    staging = Path(tempfile.mkdtemp(prefix=".admit-", dir=root / "releases"))
    try:
        target = staging / "release"
        extract_bundle(archive, target, version, commit)
        destination = root / "releases" / f"{version}-{commit}"
        for other in (root / "releases").glob(f"{version}-*"):
            if other != destination:
                raise ValueError("version was already deployed from another commit")
        if destination.exists():
            if (destination / "release.json").read_bytes() != (target / "release.json").read_bytes():
                raise ValueError("release already exists with different contents")
            for name, expected in metadata(destination)["files"].items():
                if digest(destination / name) != expected:
                    raise ValueError("installed release was modified")
        else:
            target.rename(destination)
        return destination
    finally:
        shutil.rmtree(staging)


def main():
    os.umask(0o077)
    import fcntl
    from runtime import Runtime
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    commands = parser.add_subparsers(dest="command", required=True)
    deploy = commands.add_parser("activate")
    deploy.add_argument("archive", type=Path)
    deploy.add_argument("checksum")
    deploy.add_argument("version")
    deploy.add_argument("commit")
    commands.add_parser("rollback")
    commands.add_parser("status")
    args = parser.parse_args()
    root = args.root.resolve(strict=True)
    with (root / "deploy.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        runtime = Runtime(root)
        if args.command == "activate":
            target = admit(root, args.archive, args.checksum, args.version, args.commit)
            activate(root, target, runtime)
        elif args.command == "rollback":
            rollback(root, runtime)
        target = linked(root, "current")
        if not target:
            raise ValueError("no active release")
        runtime.check(target)
        print(json.dumps({k: metadata(target)[k] for k in ("version", "commit", "schema")}))


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"deployment failed: {error}", file=sys.stderr)
        sys.exit(1)
