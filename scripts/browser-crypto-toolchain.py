#!/usr/bin/env python3
"""Prepare the pinned wasm-bindgen release inside the checkout.

The archive is checked against its pinned SHA-256 every time. CI runners set
TT_BROWSER_TOOLCHAIN_CACHE to a directory outside the cleaned checkout, so a
CDN outage does not fail a build that already holds the verified archive.
"""
import hashlib
import io
import os
from pathlib import Path
import platform
import tarfile
import time
import urllib.error
import urllib.request

VERSION = '0.2.127'
SHA256 = '61d4a7dc85acfa0d2354ccc0b8361928c7e52a746d17f28ebaa795ed3dc1614a'
NAME = f'wasm-bindgen-{VERSION}-x86_64-unknown-linux-musl'
ROOT = Path(__file__).resolve().parent.parent
ATTEMPTS = 5


def download(url):
    # GitHub's release CDN intermittently answers 5xx; client errors are final.
    for attempt in range(1, ATTEMPTS + 1):
        try:
            with urllib.request.urlopen(url, timeout=60) as response:
                return response.read(30 * 1024 * 1024)
        except urllib.error.HTTPError as error:
            if error.code < 500 or attempt == ATTEMPTS:
                raise
        except urllib.error.URLError:
            if attempt == ATTEMPTS:
                raise
        time.sleep(5 * 2 ** (attempt - 1))


def pinned(path):
    return path.exists() and hashlib.sha256(path.read_bytes()).hexdigest() == SHA256


def main():
    if platform.system() != 'Linux' or platform.machine() != 'x86_64':
        raise SystemExit('The browser build toolchain requires Linux x86_64.')
    directory = ROOT / 'output' / 'browser-toolchain'
    directory.mkdir(parents=True, exist_ok=True)
    cache = os.environ.get('TT_BROWSER_TOOLCHAIN_CACHE')
    store = Path(cache) if cache else directory
    store.mkdir(parents=True, exist_ok=True)
    archive = store / f'{NAME}.tar.gz'
    if not pinned(archive):
        url = f'https://github.com/wasm-bindgen/wasm-bindgen/releases/download/{VERSION}/{NAME}.tar.gz'
        data = download(url)
        if hashlib.sha256(data).hexdigest() != SHA256:
            raise SystemExit('Browser toolchain archive checksum mismatch.')
        partial = archive.with_suffix('.partial')
        partial.write_bytes(data)
        partial.replace(archive)
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
