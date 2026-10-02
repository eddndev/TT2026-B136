"""Build and verify immutable, versioned deployment bundles."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import tarfile
import tempfile

VERSION = re.compile(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")
COMMIT = re.compile(r"[0-9a-f]{40}")
MAX_BYTES = 512 * 1024 * 1024


def validate_version(value):
    if not VERSION.fullmatch(value):
        raise ValueError("expected canonical vMAJOR.MINOR.PATCH, without suffixes")
    return value


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def schema_digest(directory):
    hasher = hashlib.sha256()
    for path in sorted(directory.glob("*.sql")):
        hasher.update(path.name.encode() + b"\0" + path.read_bytes())
    return hasher.hexdigest()


def extract_bundle(archive, destination, version, commit):
    validate_version(version)
    if not COMMIT.fullmatch(commit):
        raise ValueError("invalid commit")
    if destination.exists() or archive.stat().st_size > MAX_BYTES:
        raise ValueError("destination exists or archive exceeds size limit")
    with tarfile.open(archive, "r:gz") as tar:
        members = tar.getmembers()
        seen = set()
        total = 0
        for item in members:
            path = PurePosixPath(item.name)
            if (path.is_absolute() or ".." in path.parts or "\\" in item.name
                    or ":" in item.name or not path.parts or item.name in seen
                    or path.as_posix() != item.name
                    or not item.isfile() or len(item.name) > 240):
                raise ValueError("unsafe or duplicate archive member")
            seen.add(item.name)
            total += item.size
        if total > MAX_BYTES or len(members) > 20000:
            raise ValueError("expanded archive exceeds limits")
        if "release.json" not in seen:
            raise ValueError("release manifest missing")
        manifest = json.load(tar.extractfile("release.json"))
        if manifest.get("version") != version or manifest.get("commit") != commit:
            raise ValueError("release identity mismatch")
        files = manifest.get("files", {})
        required = {"bin/despacho-cli", "web/index.html", "lib/libqpdf.so.30.4.1", "pki/tsa.cnf",
                    "bin/ffmpeg", "bin/ffprobe"}
        if set(files) != seen - {"release.json"} or not required.issubset(files):
            raise ValueError("release inventory mismatch")
        destination.mkdir(parents=True)
        try:
            for member in members:
                target = destination / member.name
                target.parent.mkdir(parents=True, exist_ok=True)
                with tar.extractfile(member) as source, target.open("xb") as output:
                    shutil.copyfileobj(source, output)
                target.chmod(0o700 if member.name.startswith("bin/") else 0o600)
                if member.name != "release.json" and digest(target) != files[member.name]:
                    raise ValueError("release file checksum mismatch")
            if schema_digest(destination / "migrations") != manifest.get("schema"):
                raise ValueError("schema identity mismatch")
        except BaseException:
            shutil.rmtree(destination)
            raise
    return manifest


def build(root, version, commit, library, output):
    validate_version(version)
    if not COMMIT.fullmatch(commit):
        raise ValueError("invalid commit")
    actual = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    if actual != commit:
        raise ValueError("checkout differs from release commit")
    subprocess.run(["git", "diff", "--exit-code", "HEAD"], cwd=root, check=True,
                   stdout=subprocess.DEVNULL)
    output.mkdir(parents=True, exist_ok=True)
    archive = output / f"qadra-{version}-{commit}.tar.gz"
    if archive.exists():
        raise ValueError("refusing to overwrite a release artifact")
    with tempfile.TemporaryDirectory(dir=output) as temp:
        stage = Path(temp)
        (stage / "bin").mkdir()
        shutil.copy2(root / "target/release/despacho-cli", stage / "bin/despacho-cli")
        for program in ("ffmpeg", "ffprobe"):
            shutil.copyfile(Path("/opt/tt-media/bin") / program, stage / "bin" / program)
        for source, name in ((root / "web/dist", "web"),
                             (root / "migrations", "migrations")):
            shutil.copytree(source, stage / name)
        # Only versioned PKI scripts/configuration belong in the artifact.
        tracked = set(subprocess.check_output(
            ["git", "ls-files", "pki"], cwd=root, text=True).splitlines())
        for name in tracked:
            path = stage / name
            path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(root / name, path)
        (stage / "lib").mkdir()
        for path in library.parent.glob("*.so*"):
            if path.is_file():
                shutil.copyfile(path, stage / "lib" / path.name)
        identity = {"version": version, "commit": commit}
        (stage / "web/version.json").write_text(json.dumps(identity) + "\n")
        files = {p.relative_to(stage).as_posix(): digest(p)
                 for p in sorted(stage.rglob("*")) if p.is_file()}
        manifest = {**identity, "schema": schema_digest(stage / "migrations"), "files": files}
        (stage / "release.json").write_text(json.dumps(manifest, indent=2) + "\n")
        with tarfile.open(archive, "w:gz") as tar:
            for path in sorted(stage.rglob("*")):
                if path.is_file():
                    tar.add(path, arcname=path.relative_to(stage).as_posix(), recursive=False)
    archive.with_suffix(archive.suffix + ".sha256").write_text(
        f"{digest(archive)}  {archive.name}\n")
    return archive


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version")
    parser.add_argument("commit")
    parser.add_argument("library", type=Path)
    parser.add_argument("--output", type=Path, default=Path("output/releases"))
    args = parser.parse_args()
    print(build(Path.cwd(), args.version, args.commit, args.library, args.output))
