#!/usr/bin/env python3
"""Binary statements and the facts index of the evidence-verify fixtures.

Called by scripts/evidence-verify-fixtures.sh. It uses only the Python
standard library and the openssl command line.

The "statements" command writes the canonical certificate binding
(DOCBIND1, 142 bytes) and document consent (DOCSIGN1, 258 bytes) with
the byte layouts of crates/domain/src/document_signing.rs and its
consent module. The "index" command writes fixtures.json: the SHA-256
of every fixture file and the facts that tests need, read back from the
generated artifacts with openssl.

Rationale: docs/adr/0085-in-browser-evidence-validation.md.
"""

import argparse
import calendar
import hashlib
import json
import os
import re
import subprocess
import sys
import time
import uuid

DEPLOYMENT_ID = uuid.UUID("6f1c2a3b-4d5e-4f60-8a71-b2c3d4e5f601")
ACCOUNT_ID = uuid.UUID("6f1c2a3b-4d5e-4f60-8a71-b2c3d4e5f602")
BINDING_ID = uuid.UUID("6f1c2a3b-4d5e-4f60-8a71-b2c3d4e5f603")
OPERATION_ID = uuid.UUID("6f1c2a3b-4d5e-4f60-8a71-b2c3d4e5f604")
CASE_ID = uuid.UUID("6f1c2a3b-4d5e-4f60-8a71-b2c3d4e5f605")
DOCUMENT_ID = uuid.UUID("6f1c2a3b-4d5e-4f60-8a71-b2c3d4e5f606")
TRUST_REVISION = 3
ACCOUNT_REVISION = 7
ACCOUNT_GENERATION = 2
DOCUMENT_VERSION = 1

BINDING_PURPOSE = b"DOCBIND1"
CONSENT_PURPOSE = b"DOCSIGN1"
BINDING_LENGTH = 142
CONSENT_LENGTH = 258

OPENSSL_DATE = "%b %d %H:%M:%S %Y GMT"


def sha256(data):
    return hashlib.sha256(data).digest()


def read(path):
    with open(path, "rb") as handle:
        return handle.read()


def statement_prefix(purpose, root_fingerprint, leaf_fingerprint):
    data = (
        purpose
        + bytes([1, 1])
        + DEPLOYMENT_ID.bytes
        + root_fingerprint
        + TRUST_REVISION.to_bytes(4, "big")
        + ACCOUNT_ID.bytes
        + ACCOUNT_REVISION.to_bytes(8, "big")
        + ACCOUNT_GENERATION.to_bytes(8, "big")
        + BINDING_ID.bytes
        + leaf_fingerprint
    )
    assert len(data) == BINDING_LENGTH
    return data


def command_statements(args):
    root = sha256(read(args.root))
    leaf = sha256(read(args.holder))
    binding = statement_prefix(BINDING_PURPOSE, root, leaf)
    consent = (
        statement_prefix(CONSENT_PURPOSE, root, leaf)
        + OPERATION_ID.bytes
        + CASE_ID.bytes
        + DOCUMENT_ID.bytes
        + DOCUMENT_VERSION.to_bytes(4, "big")
        + sha256(read(args.document))
        + sha256(read(args.holder_signature))
    )
    assert len(consent) == CONSENT_LENGTH
    with open(args.binding_out, "wb") as handle:
        handle.write(binding)
    with open(args.consent_out, "wb") as handle:
        handle.write(consent)


def decode_statement(data):
    """Return the named fields of a binding or consent statement."""
    def number(offset, size):
        return int.from_bytes(data[offset:offset + size], "big")

    def ident(offset):
        return str(uuid.UUID(bytes=data[offset:offset + 16]))

    fields = {
        "purpose": data[:8].decode("ascii"),
        "purpose_version": data[8],
        "format_version": data[9],
        "deployment_id": ident(10),
        "root_sha256": data[26:58].hex(),
        "trust_revision": number(58, 4),
        "account_id": ident(62),
        "account_revision": number(78, 8),
        "account_generation": number(86, 8),
        "binding_id": ident(94),
        "leaf_sha256": data[110:142].hex(),
        "length": len(data),
    }
    if len(data) == CONSENT_LENGTH:
        fields.update({
            "operation_id": ident(142),
            "case_id": ident(158),
            "document_id": ident(174),
            "document_version": number(190, 4),
            "document_sha256": data[194:226].hex(),
            "document_signature_sha256": data[226:258].hex(),
        })
    return fields


def openssl(*args):
    result = subprocess.run(
        ["openssl", *args], check=True, capture_output=True, text=True
    )
    return result.stdout


def unix(text):
    return calendar.timegm(time.strptime(text.strip(), OPENSSL_DATE))


def serial_hex(text):
    return text.strip().upper().lstrip("0") or "0"


def colon_hex(text):
    return text.strip().replace(":", "").lower()


def certificate_facts(path):
    out = openssl(
        "x509", "-inform", "DER", "-in", path, "-noout", "-serial",
        "-startdate", "-enddate", "-subject", "-issuer", "-nameopt", "RFC2253",
        "-ext", "subjectKeyIdentifier,authorityKeyIdentifier",
    )
    lines = out.splitlines()
    values = dict(line.split("=", 1) for line in lines if "=" in line
                  and line.split("=", 1)[0] in
                  ("serial", "notBefore", "notAfter", "subject", "issuer"))
    facts = {
        "serial_hex": serial_hex(values["serial"]),
        "not_before": unix(values["notBefore"]),
        "not_after": unix(values["notAfter"]),
        "subject": values["subject"],
        "issuer": values["issuer"],
    }
    for label, key in (("Subject Key Identifier", "subject_key_id"),
                       ("Authority Key Identifier", "authority_key_id")):
        for index, line in enumerate(lines):
            if label in line:
                value = lines[index + 1].strip()
                facts[key] = colon_hex(value.removeprefix("keyid:"))
    text = openssl("x509", "-inform", "DER", "-in", path, "-noout", "-text")
    bits = re.search(r"Public-Key: \((\d+) bit\)", text)
    facts["key_bits"] = int(bits.group(1))
    return facts


def crl_facts(path):
    text = openssl("crl", "-inform", "DER", "-in", path, "-noout", "-text")
    this_update = re.search(r"Last Update: (.+)", text).group(1)
    next_update = re.search(r"Next Update: (.+)", text).group(1)
    number = re.search(r"CRL Number:\s*\n\s*(\S+)", text).group(1)
    number = int(number, 16) if number.startswith("0x") else int(number)
    revoked = [
        {"serial_hex": serial_hex(serial), "revocation": unix(date)}
        for serial, date in re.findall(
            r"Serial Number: (\S+)\s*\n\s*Revocation Date: (.+)", text)
    ]
    if "CRL entry extensions" in text:
        raise SystemExit(f"{path}: unexpected CRL entry extensions")
    return {
        "this_update": unix(this_update),
        "next_update": unix(next_update),
        "crl_number": number,
        "revoked": revoked,
    }


def imprint_from_dump(text):
    """Read the hex bytes of the "Message data" dump of ts -text."""
    block = text.split("Message data:", 1)[1]
    data = ""
    for line in block.splitlines()[1:]:
        match = re.match(r"\s*[0-9a-f]{4} - (.*)", line)
        if not match:
            break
        data += re.sub(r"[^0-9a-f]", "", match.group(1).split("   ")[0])
    return data


def token_facts(path, form):
    in_args = ["-token_in"] if form == "token" else []
    text = openssl("ts", "-reply", "-in", path, *in_args, "-text")
    accuracy = re.search(r"Accuracy: (0x[0-9A-Fa-f]+|\d+) seconds", text)
    serial = re.search(r"Serial number: (\S+)", text).group(1)
    parsed = openssl("asn1parse", "-inform", "DER", "-in", path)
    if re.search(r":id-smime-aa-signingCertificateV2\s*$", parsed, re.M):
        ess = "signing-certificate-v2"
    elif re.search(r":id-smime-aa-signingCertificate\s*$", parsed, re.M):
        ess = "signing-certificate"
    else:
        ess = "absent"
    status = None
    if form == "response":
        status = re.search(r"Status: (.+)", text).group(1).strip().rstrip(".")
        status = status.lower()
    return {
        "status": status,
        "gen_time": unix(re.search(r"Time stamp: (.+)", text).group(1)),
        "accuracy_seconds": int(accuracy.group(1), 0),
        "policy": re.search(r"Policy OID: (\S+)", text).group(1),
        "ess": ess,
        "covered_sha256": imprint_from_dump(text),
        "serial_hex": serial_hex(serial.removeprefix("0x")),
        "ordering": re.search(r"Ordering: (\S+)", text).group(1) == "yes",
        "nonce_present": "Nonce: unspecified" not in text,
    }


def expect(name, label, actual, expected):
    if actual != expected:
        raise SystemExit(f"{name}: {label} is {actual!r}, expected {expected!r}")


def command_index(args):
    files = {}
    for name in sorted(os.listdir(args.dir)):
        if name in ("fixtures.json", "README.md"):
            continue
        data = read(os.path.join(args.dir, name))
        files[name] = {"sha256": sha256(data).hex(), "size": len(data)}
    index = {
        "generator": "scripts/evidence-verify-fixtures.sh",
        "generated_at": int(args.generated_at),
        "files": files,
        "certificates": {},
        "crls": {},
        "signatures": {},
        "statements": {},
        "tokens": {},
        "document": {"file": "document.bin",
                     "sha256": files["document.bin"]["sha256"]},
    }
    with open(args.facts, encoding="ascii") as handle:
        rows = [line.split() for line in handle if line.strip()]
    for row in rows:
        kind, name = row[0], row[1]
        path = os.path.join(args.dir, name)
        if kind == "cert":
            facts = certificate_facts(path)
            facts.update({"profile": row[2],
                          "issuer_file": None if row[3] == "-" else row[3],
                          "defect": None if row[4] == "-" else row[4],
                          "sha256": files[name]["sha256"]})
            index["certificates"][name] = facts
        elif kind == "crl":
            facts = crl_facts(path)
            facts["issuer_file"] = row[2]
            index["crls"][name] = facts
        elif kind == "sig":
            index["signatures"][name] = {"signer": row[2], "over": row[3],
                                         "algorithm": "rsa-pkcs1v15-sha256"}
        elif kind == "statement":
            facts = decode_statement(read(path))
            facts.update({"signature": row[2], "signer": row[3]})
            index["statements"][name] = facts
        elif kind == "token":
            form, signer, anchor, ess, digest, covers, policy = row[2:9]
            facts = token_facts(path, form)
            expect(name, "ess", facts["ess"], ess)
            expect(name, "covered digest", facts["covered_sha256"], digest)
            expect(name, "policy", facts["policy"], policy)
            expect(name, "accuracy", facts["accuracy_seconds"], 1)
            expect(name, "ordering", facts["ordering"], False)
            expect(name, "nonce", facts["nonce_present"], True)
            if form == "response":
                expect(name, "status", facts["status"], "granted")
            facts.update({"form": form, "signer": signer, "anchor": anchor,
                          "covers": None if covers == "-" else covers})
            index["tokens"][name] = facts
        else:
            raise SystemExit(f"unknown fact kind: {kind}")
    for name in files:
        if name.endswith((".der", ".tsr")) and not any(
                name in index[group] for group in
                ("certificates", "crls", "tokens")):
            raise SystemExit(f"{name}: no facts recorded")
    with open(args.out, "w", encoding="ascii") as handle:
        json.dump(index, handle, indent=2, sort_keys=True, ensure_ascii=True)
        handle.write("\n")


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)
    statements = commands.add_parser("statements")
    for option in ("--root", "--holder", "--document", "--holder-signature",
                   "--binding-out", "--consent-out"):
        statements.add_argument(option, required=True)
    index = commands.add_parser("index")
    for option in ("--dir", "--facts", "--generated-at", "--out"):
        index.add_argument(option, required=True)
    args = parser.parse_args(argv)
    if args.command == "statements":
        command_statements(args)
    else:
        command_index(args)


if __name__ == "__main__":
    main(sys.argv[1:])
