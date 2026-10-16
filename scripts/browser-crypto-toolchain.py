#!/usr/bin/env python3
"""Prepare the pinned wasm-bindgen release inside the checkout."""
import hashlib
import io
from pathlib import Path
import platform
import tarfile
import urllib.request

VERSION = '0.2.127'
SHA256 = '61d4a7dc85acfa0d2354ccc0b8361928c7e52a746d17f28ebaa795ed3dc1614a'
NAME = f'wasm-bindgen-{VERSION}-x86_64-unknown-linux-musl'
ROOT = Path(__file__).resolve().parent.parent


def main():
    if platform.system() != 'Linux' or platform.machine() != 'x86_64':
        raise SystemExit('The browser build toolchain requires Linux x86_64.')
    directory = ROOT / 'output' / 'browser-toolchain'
    directory.mkdir(parents=True, exist_ok=True)
    archive = directory / f'{NAME}.tar.gz'
    if not archive.exists():
        url = f'https://github.com/wasm-bindgen/wasm-bindgen/releases/download/{VERSION}/{NAME}.tar.gz'
        with urllib.request.urlopen(url, timeout=60) as response:
            data = response.read(30 * 1024 * 1024)
        if hashlib.sha256(data).hexdigest() != SHA256:
            raise SystemExit('Browser toolchain archive checksum mismatch.')
        archive.write_bytes(data)
    data = archive.read_bytes()
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise SystemExit('Browser toolchain archive checksum mismatch.')
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as bundle:
        member = bundle.getmember(f'{NAME}/wasm-bindgen')
        if not member.isfile() or member.size > 30 * 1024 * 1024:
            raise SystemExit('Invalid browser toolchain executable.')
        binary = bundle.extractfile(member).read()
    target = directory / 'wasm-bindgen'
    if not target.exists() or target.read_bytes() != binary:
        target.write_bytes(binary)
    target.chmod(0o700)
    print(target)


if __name__ == '__main__':
    main()
