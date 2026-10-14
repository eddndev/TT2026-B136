"""Bounded OpenSSL state observations for one public-request issuance."""
import hashlib
import os
import re
import stat

import crl_journal as journal


LIMITS = {"ca.crt.pem": 16384, "index.txt": 16 * 1024 * 1024,
          "serial": 64, "crlnumber": 64, "index.txt.attr": 1024}
OPTIONAL = {"index.txt.attr"}
for _name in ("index.txt", "serial", "crlnumber", "index.txt.attr"):
    for _suffix in (".old", ".new"):
        LIMITS[_name + _suffix] = LIMITS[_name]
        OPTIONAL.add(_name + _suffix)


class CsrSubjectPolicyMismatch(ValueError):
    """A verified public CSR does not match the adopted issuer subject policy."""
    def __init__(self, admitted):
        super().__init__("CSR subject must match the adopted CA country and organization and include a common name")
        self.admitted = dict(admitted)


def hex_digest(value):
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def digest(value):
    return hashlib.sha256(value).hexdigest()


def serial(value):
    if not isinstance(value, str) or not re.fullmatch(r"[0-9A-Fa-f]{1,40}", value):
        raise ValueError("invalid CA serial")
    number = int(value, 16)
    if not 0 < number < 2**159 - 1:
        raise ValueError("invalid or exhausted CA serial")
    return format(number, "X")


def read(path, maximum, *, empty=False):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError("CA material must be a regular file")
        raw = stream.read(maximum + 1)
    if len(raw) > maximum or (not empty and not raw):
        raise ValueError("CA material is empty or exceeds its bound")
    return raw


def absent(path):
    try:
        path.lstat()
    except FileNotFoundError:
        return True
    return False


def directory(path):
    if not stat.S_ISDIR(path.lstat().st_mode):
        raise ValueError("CA directory must not be a link or another file type")


def snapshot(ca):
    files = {}
    for name, maximum in LIMITS.items():
        try:
            raw = read(ca / name, maximum, empty=name.startswith("index.txt"))
        except FileNotFoundError:
            if name not in OPTIONAL:
                raise
            files[name] = None
        else:
            files[name] = digest(raw)
    current = serial(read(ca / "serial", 64).decode("ascii").strip())
    return {"files": files, "serial": current}


def index(ca):
    raw = read(ca / "index.txt", LIMITS["index.txt"], empty=True)
    rows, seen = [], set()
    for line in raw.splitlines(keepends=True):
        fields = line.rstrip(b"\n").split(b"\t")
        if (not line.endswith(b"\n") or len(fields) != 6
                or fields[0] not in (b"V", b"R", b"E")):
            raise ValueError("CA index row is malformed")
        number = serial(fields[3].decode("ascii"))
        if number in seen:
            raise ValueError("CA index repeats a serial")
        seen.add(number)
        rows.append((line, number, fields))
    return rows


def leaf_paths(ca, operation_id, reserved):
    # OpenSSL names newcerts with even-length uppercase hexadecimal.
    filename = reserved.zfill(len(reserved) + len(reserved) % 2)
    return ca / "newcerts" / (filename + ".pem"), ca / "certs" / (operation_id + ".crt.pem")


def admit(ca, request, executor):
    root = read(ca / "ca.crt.pem", 16384)
    try:
        facts = executor.admit(request["csr_pem"].encode("ascii"), root)
    except CsrSubjectPolicyMismatch as error:
        admission_facts(request, error.admitted)
        raise
    return admission_facts(request, facts)


def admission_facts(request, facts):
    names = {"csr_sha256", "spki_sha256", "root_fingerprint"}
    if (not isinstance(facts, dict) or set(facts) != names
            or not all(hex_digest(value) for value in facts.values())
            or facts["csr_sha256"] != digest(request["csr_pem"].encode("ascii"))
            or facts["root_fingerprint"] != request["expected_root_fingerprint"]):
        raise ValueError("request admission or adopted CA identity differs")
    return dict(facts)


def pristine(ca, operation_id, before):
    paths = leaf_paths(ca, operation_id, before["serial"])
    return snapshot(ca) == before and all(absent(path) for path in paths)


def prepare(ca, operation_id):
    before = snapshot(ca)
    if any(before["files"][name] is not None for name in LIMITS if name.endswith(".new")):
        raise ValueError("CA has unfinished OpenSSL material")
    if any(int(number, 16) >= int(before["serial"], 16) for _, number, _ in index(ca)):
        raise ValueError("CA serial does not follow its index")
    if not all(absent(path) for path in leaf_paths(ca, operation_id, before["serial"])):
        raise ValueError("planned issuance already has certificate material")
    return before


def recover(ca, operation_id, record, executor):
    before, request = record["before"], record["request"]
    reserved = before["serial"]
    current = snapshot(ca)
    rows = index(ca)
    issued = [fields for _, number, fields in rows if number == reserved]
    remaining = b"".join(line for line, number, _ in rows if number != reserved)
    if (len(issued) != 1 or issued[0][0] != b"V" or issued[0][2]
            or digest(remaining) != before["files"]["index.txt"]
            or int(current["serial"], 16) != int(reserved, 16) + 1
            or current["files"]["crlnumber"] != before["files"]["crlnumber"]
            or current["files"]["ca.crt.pem"] != before["files"]["ca.crt.pem"]
            or any(current["files"][name] is not None for name in LIMITS if name.endswith(".new"))):
        raise RuntimeError("issuance state is partial or differs from its reserved serial")
    original, named = leaf_paths(ca, operation_id, reserved)
    if absent(original):
        raise RuntimeError("reserved certificate is absent; issuance cannot be inferred")
    certificate = read(original, 16384)
    root = read(ca / "ca.crt.pem", 16384)
    facts = executor.inspect_certificate(certificate, root)
    names = {"serial", "fingerprint", "spki_sha256", "root_fingerprint", "purpose"}
    if (not isinstance(facts, dict) or set(facts) != names
            or serial(facts["serial"]) != reserved or not hex_digest(facts["fingerprint"])
            or facts["spki_sha256"] != record["admitted"]["spki_sha256"]
            or facts["root_fingerprint"] != record["admitted"]["root_fingerprint"]
            or facts["purpose"] != request["purpose"]):
        raise ValueError("reserved certificate does not match its request, profile or CA")
    if not absent(named):
        named_facts = executor.inspect_certificate(read(named, 16384), root)
        if named_facts != facts:
            raise ValueError("named certificate differs from the reserved certificate")
    # OpenSSL success is not a durable receipt until its material is flushed.
    for path in [ca / name for name in LIMITS if current["files"][name] is not None] + [original, named]:
        if absent(path):
            continue
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        try:
            if not stat.S_ISREG(os.fstat(descriptor).st_mode):
                raise ValueError("CA material changed its file type")
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
    for path in (ca, ca / "newcerts", ca / "certs"):
        journal.sync_directory(path)
    return {"serial": reserved, "fingerprint": facts["fingerprint"],
            "purpose": request["purpose"], "certificate_pem": certificate.decode("ascii")}
