"""Read-only public CA inventory with explicit local and unknown published state."""
from contextlib import contextmanager
import fcntl
import os
from pathlib import Path
import re
import stat

import issuance_material as material


FACT_FIELDS = {"serial", "fingerprint", "spki_sha256", "root_fingerprint", "purpose",
               "subject", "not_before", "not_after"}
LOCAL_STATUS = {b"V": "issued", b"R": "revoked", b"E": "expired"}


@contextmanager
def _read_locked(root):
    # Readers share the writers' lock without creating or modifying any file.
    descriptor = os.open(root / "deploy.lock", os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError("CA lock must be a regular file")
        fcntl.flock(stream.fileno(), fcntl.LOCK_SH | fcntl.LOCK_NB)
        yield


def _facts(certificate, root_certificate, executor):
    facts = executor.describe_certificate(certificate, root_certificate)
    if not isinstance(facts, dict) or set(facts) != FACT_FIELDS:
        raise ValueError("certificate inspection fields differ")
    serial = material.serial(facts["serial"])
    if (not all(material.hex_digest(facts[name])
                for name in ("fingerprint", "spki_sha256", "root_fingerprint"))
            or facts["purpose"] not in ("partner", "document-signing", "time-stamping")
            or not isinstance(facts["subject"], str)
            or not 0 < len(facts["subject"].encode("utf-8")) <= 4096
            or type(facts["not_before"]) is not int or type(facts["not_after"]) is not int
            or facts["not_before"] > facts["not_after"]):
        raise ValueError("certificate inspection metadata is invalid")
    return {**facts, "serial": serial}


def _public_paths(ca):
    # The serial archive wins byte selection when a legacy named copy also exists.
    for name in ("newcerts", "certs"):
        directory = ca / name
        if material.absent(directory):
            continue
        material.directory(directory)
        for path in sorted(directory.iterdir(), key=lambda entry: entry.name):
            if name == "newcerts":
                if re.fullmatch(r"[0-9A-Fa-f]{1,40}\.pem", path.name):
                    yield path, material.serial(path.name[:-4])
            elif path.name.endswith(".crt.pem"):
                yield path, None


def _certificates(root, executor):
    material.directory(root / "data")
    ca = root / "data/ca"
    material.directory(ca)
    statuses = {serial: LOCAL_STATUS[fields[0]] for _, serial, fields in material.index(ca)}
    root_certificate = material.read(ca / "ca.crt.pem", 16384)
    found, root_fingerprint = {}, None
    for path, filename_serial in _public_paths(ca):
        certificate = material.read(path, 16384)
        facts = _facts(certificate, root_certificate, executor)
        serial = facts["serial"]
        if filename_serial is not None and filename_serial != serial:
            raise ValueError("certificate serial differs from its archive filename")
        if serial not in statuses:
            raise ValueError("public certificate has no local CA index entry")
        if root_fingerprint is not None and root_fingerprint != facts["root_fingerprint"]:
            raise ValueError("public certificate inspections disagree on CA identity")
        root_fingerprint = facts["root_fingerprint"]
        if serial in found:
            if found[serial][0] != facts:
                raise ValueError("public certificates conflict for the same serial")
        else:
            found[serial] = (facts, certificate, statuses[serial])
    if set(found) != set(statuses):
        raise ValueError("local CA index references missing public certificate material")
    return found


def _row(facts, local_status, now_unix):
    if now_unix < facts["not_before"]:
        validity = "not-yet-valid"
    elif now_unix > facts["not_after"]:
        validity = "expired"
    else:
        validity = "valid"
    return {name: facts[name] for name in (
        "serial", "subject", "purpose", "not_before", "not_after", "fingerprint", "root_fingerprint"
    )} | {"validity": validity, "local_status": local_status, "published_status": "unknown"}


def list_certificates(root, executor, *, now_unix):
    """Return numeric-serial-ordered metadata using an injected Unix clock value.

    The root is the configured deployment directory. Its existing deploy.lock
    synchronizes this observation with issuance and CRL operations. V/R/E index
    states map to issued/revoked/expired; validity only describes signed dates.
    Published CRL status is unknown and cannot be inferred from local material.
    """
    if type(now_unix) is not int:
        raise ValueError("inventory time must be an integer Unix timestamp")
    root = Path(root).resolve(strict=True)
    with _read_locked(root):
        certificates = _certificates(root, executor)
        return [_row(certificates[serial][0], certificates[serial][2], now_unix)
                for serial in sorted(certificates, key=lambda value: int(value, 16))]


def download_certificate(root, serial, fingerprint, executor):
    """Return exact public PEM bytes selected by serial and its inspected DER hash.

    Prefer the serial archive to a legacy named copy of the same certificate.
    A missing serial raises LookupError; a changed fingerprint or inconsistent
    public inventory raises ValueError. No caller-supplied path is accepted.
    """
    serial = material.serial(serial)
    if not material.hex_digest(fingerprint):
        raise ValueError("certificate fingerprint must be lowercase SHA-256 hexadecimal")
    root = Path(root).resolve(strict=True)
    with _read_locked(root):
        certificates = _certificates(root, executor)
        if serial not in certificates:
            raise LookupError("certificate serial is absent from the local inventory")
        facts, certificate, _ = certificates[serial]
        if facts["fingerprint"] != fingerprint:
            raise ValueError("certificate fingerprint differs from the requested identity")
        return certificate
