"""Read and stage credential trust without modifying the live CA database."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import uuid

from runtime import environment, run, sync_directory


FIELDS = ("root_der", "crl_der", "root_fingerprint", "crl_digest", "crl_number",
          "crl_this_update", "crl_next_update", "valid_from", "valid_until")


def _bounded(path, maximum):
    with Path(path).open("rb") as stream:
        data = stream.read(maximum + 1)
    if not data or len(data) > maximum:
        raise ValueError("public credential material is empty or too large")
    return data


def _integer(value, maximum):
    if type(value) is int:
        result = value
    elif isinstance(value, str) and re.fullmatch(r"[0-9]+", value):
        result = int(value)
    else:
        raise ValueError("invalid credential trust integer")
    if result < 0 or result > maximum:
        raise ValueError("credential trust integer out of range")
    return result


def read_head(root):
    query = """SELECT row_to_json(head) FROM (
        SELECT a.deployment_id, r.revision,
               encode(a.root_der, 'hex') AS root_der,
               encode(r.crl_der, 'hex') AS crl_der,
               encode(a.root_fingerprint, 'hex') AS root_fingerprint,
               encode(r.crl_digest, 'hex') AS crl_digest,
               r.crl_number::text AS crl_number, r.crl_this_update,
               r.crl_next_update, r.valid_from, r.valid_until
        FROM participant_credential_authority a
        JOIN participant_credential_trust_revisions r USING (deployment_id)
        ORDER BY r.revision DESC LIMIT 1
    ) AS head"""
    result = run(["psql", "-XAt", "-v", "ON_ERROR_STOP=1", "-c", query],
                 env=environment(root, admin=True), capture_output=True, text=True, timeout=15)
    try:
        head = json.loads(result.stdout)
        if not isinstance(head, dict) or set(head) != {*FIELDS, "deployment_id", "revision"}:
            raise ValueError("incomplete credential trust")
        if str(uuid.UUID(head["deployment_id"])) != head["deployment_id"]:
            raise ValueError("invalid deployment identity")
        head["revision"] = _integer(head["revision"], 2**32 - 1)
        if head["revision"] == 0:
            raise ValueError("credential trust is not initialized")
        head["crl_number"] = _integer(head["crl_number"], 2**64 - 1)
        for field in ("crl_this_update", "crl_next_update", "valid_from", "valid_until"):
            head[field] = _integer(head[field], 253402300799)
        for name, digest, maximum in (("root_der", "root_fingerprint", 16384),
                                      ("crl_der", "crl_digest", 1048576)):
            encoded = head[name]
            if (not isinstance(encoded, str) or not re.fullmatch(r"[0-9a-f]+", encoded)
                    or len(encoded) > maximum * 2):
                raise ValueError("invalid public credential material")
            data = bytes.fromhex(encoded)
            if hashlib.sha256(data).hexdigest() != head[digest]:
                raise ValueError("credential trust digest differs")
        if not (head["crl_this_update"] <= head["valid_from"] <= head["valid_until"]
                <= head["crl_next_update"] and head["crl_this_update"] < head["crl_next_update"]):
            raise ValueError("invalid credential trust validity")
    except (TypeError, ValueError, KeyError, AttributeError) as error:
        raise ValueError("persisted credential trust is absent or inconsistent") from error
    return head


def _openssl(arguments, text=False):
    return run(["openssl", *arguments], env={**os.environ, "LC_ALL": "C", "TZ": "UTC"},
               capture_output=True, text=text, timeout=15).stdout


def _timestamp(value):
    match = re.fullmatch(r"([A-Z][a-z]{2})\s+([0-9]{1,2}) ([0-9]{2}):([0-9]{2}):([0-9]{2}) ([0-9]{4}) GMT", value)
    if not match:
        raise ValueError("unexpected credential validity format")
    month, day, hour, minute, second, year = match.groups()
    months = "Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec".split()
    return int(datetime.datetime(int(year), months.index(month) + 1, int(day), int(hour),
                                 int(minute), int(second), tzinfo=datetime.timezone.utc).timestamp())


def inspect(root_cert, crl):
    _bounded(root_cert, 16384)
    _bounded(crl, 1048576)
    _openssl(["crl", "-in", crl, "-noout", "-verify", "-CAfile", root_cert])
    root_der = _openssl(["x509", "-in", root_cert, "-outform", "DER"])
    crl_der = _openssl(["crl", "-in", crl, "-outform", "DER"])
    times = _openssl(["crl", "-in", crl, "-noout", "-lastupdate", "-nextupdate", "-crlnumber"], True)
    fields = dict(line.split("=", 1) for line in times.splitlines())
    number = fields["crlNumber"]
    if not re.fullmatch(r"0x[0-9A-Fa-f]+", number):
        raise ValueError("CRL has no supported number")
    number = int(number, 16)
    if number > 2**64 - 1:
        raise ValueError("CRL number exceeds the publication range")
    dates = dict(line.split("=", 1) for line in _openssl(
        ["x509", "-in", root_cert, "-noout", "-dates"], True).splitlines())
    first, last = _timestamp(fields["lastUpdate"]), _timestamp(fields["nextUpdate"])
    valid_from, valid_until = max(first, _timestamp(dates["notBefore"])), min(last, _timestamp(dates["notAfter"]))
    if not 0 <= first < last <= 253402300799 or valid_from > valid_until:
        raise ValueError("credential material has no valid interval")
    detail = _openssl(["crl", "-in", crl, "-noout", "-text"], True)
    serials = re.findall(r"^\s+Serial Number: ([0-9A-Fa-f]+)$", detail, flags=re.MULTILINE)
    return {"root_der": root_der.hex(), "crl_der": crl_der.hex(),
            "root_fingerprint": hashlib.sha256(root_der).hexdigest(),
            "crl_digest": hashlib.sha256(crl_der).hexdigest(), "crl_number": number,
            "crl_this_update": first, "crl_next_update": last,
            "valid_from": valid_from, "valid_until": valid_until,
            "revoked_serials": sorted({format(int(s, 16), "X") for s in serials})}


def _private_copy(source, destination, maximum):
    with Path(source).open("rb") as stream:
        data = stream.read(maximum + 1)
    if len(data) > maximum:
        raise ValueError("CA maintenance input exceeds capacity")
    descriptor = os.open(destination, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())


def generate(root, target, operation_dir):
    ca = root / "data/ca"
    counter = (ca / "crlnumber").read_text().strip()
    if not re.fullmatch(r"[0-9A-Fa-f]{1,16}", counter) or int(counter, 16) >= 2**64 - 1:
        raise ValueError("invalid or exhausted CA CRL counter")
    staged = operation_dir / "ca"
    staged.mkdir(mode=0o700)
    for name in ("certs", "newcerts", "crl"):
        (staged / name).mkdir(mode=0o700)
    for name, maximum in (("index.txt", 16 * 1024 * 1024), ("serial", 64), ("crlnumber", 64),
                          ("ca.crt.pem", 16384)):
        _private_copy(ca / name, staged / name, maximum)
    if (ca / "index.txt.attr").exists():
        _private_copy(ca / "index.txt.attr", staged / "index.txt.attr", 1024)
    candidate = operation_dir / "candidate.pem"
    descriptor = os.open(candidate, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    os.close(descriptor)
    run(["openssl", "ca", "-batch", "-config", target / "pki/openssl.cnf", "-gencrl",
         "-keyfile", ca / "private/ca.key.pem", "-cert", ca / "ca.crt.pem", "-out", candidate],
        env={**os.environ, "PKI_CA_DIR": str(staged), "LC_ALL": "C", "TZ": "UTC"},
        capture_output=True, timeout=30, umask=0o077)
    next_counter = (staged / "crlnumber").read_text().strip()
    if (not re.fullmatch(r"[0-9A-Fa-f]{1,16}", next_counter)
            or int(next_counter, 16) != int(counter, 16) + 1):
        raise ValueError("staged CRL counter did not advance exactly once")
    with candidate.open("rb") as stream:
        os.fsync(stream.fileno())
    sync_directory(operation_dir)
    return {"path": candidate, "next_counter": next_counter}


def publish(root, target, candidate, expected_revision):
    env = environment(root, admin=True)
    env["LD_LIBRARY_PATH"] = str(target / "lib")
    run([target / "bin/despacho-cli", "credential-trust", "publish", "--root-cert",
         root / "data/ca/ca.crt.pem", "--crl", candidate,
         "--expected-revision", str(expected_revision)], env=env, cwd=root / "data",
        capture_output=True, timeout=60)
