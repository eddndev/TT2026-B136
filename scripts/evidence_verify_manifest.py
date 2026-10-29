#!/usr/bin/env python3
"""The evidence manifest fixture of crates/evidence-verify.

Called by scripts/evidence-verify-fixtures.sh with python3 -I. It uses only
the Python standard library.

It writes manifiesto.json for the individual signing v2 package that the
tests of crates/evidence-verify assemble from the fixture files: each role
of the manifest names the package entry the server writes for it and the
fixture file the tests place there. The identifiers, document version and
document digest are read back from the consent statement, so the manifest
and the statements cannot drift apart. The output is the RFC 8785 form of
the admitted subset: object keys sorted, no whitespace, strings escaped
only where RFC 8785 requires, artifacts sorted by entry name, and no
trailing newline.

Rationale: the section "The evidence manifest" of
docs/adr/0085-in-browser-evidence-validation.md.
"""

import argparse
import hashlib
import json
import os
import sys
import uuid

CONSENT_LENGTH = 258

# Role, package entry (DOCUMENT stands for the document name) and the
# fixture file the tests place in that entry.
ROLES = (
    ("document", "DOCUMENT", "document.bin"),
    ("document-signature", "DOCUMENT.sig", "holder.sig"),
    ("document-timestamp", "DOCUMENT.tsr", "token-ess-v1.tsr"),
    ("signer-certificate", "certificado.pem", "holder.pem"),
    ("issuer-certificate", "ca.pem", "root.pem"),
    ("issuer-crl", "crl.pem", "root.crl.pem"),
    ("tsa-chain", "tsa-chain.pem", "root.pem"),
    ("tsa-issuer-crl", "tsa-crl.pem", "root.crl.pem"),
    ("binding-statement", "certificate-binding.bin", "certificate-binding.bin"),
    ("binding-signature", "certificate-binding.sig", "certificate-binding.sig"),
    ("binding-root", "binding-root.der", "root.der"),
    ("binding-crl", "binding-crl.der", "root.crl.der"),
    ("consent-statement", "document-consent.bin", "document-consent.bin"),
    ("consent-signature", "document-consent.sig", "document-consent.sig"),
)

# Members of signers[].entries and the role each one names.
SIGNER_ENTRIES = (
    ("document_signature", "document-signature"),
    ("signer_certificate", "signer-certificate"),
    ("binding_statement", "binding-statement"),
    ("binding_signature", "binding-signature"),
    ("binding_root", "binding-root"),
    ("binding_crl", "binding-crl"),
    ("consent_statement", "consent-statement"),
    ("consent_signature", "consent-signature"),
)

MAX_INTEGER = 2 ** 53 - 1


def read(path):
    with open(path, "rb") as handle:
        return handle.read()


def ident(data, offset):
    return str(uuid.UUID(bytes=data[offset:offset + 16]))


def build(directory, document, registered_at):
    consent = read(os.path.join(directory, "document-consent.bin"))
    if len(consent) != CONSENT_LENGTH:
        raise SystemExit("document-consent.bin has the wrong length")
    if not 0 <= registered_at <= MAX_INTEGER:
        raise SystemExit("binding_registered_at is outside the manifest range")
    entries = {}
    artifacts = []
    for role, entry, source in ROLES:
        name = entry.replace("DOCUMENT", document)
        data = read(os.path.join(directory, source))
        entries[role] = name
        artifacts.append({
            "entry": name,
            "role": role,
            "sha256": hashlib.sha256(data).hexdigest(),
            "size": len(data),
        })
    artifacts.sort(key=lambda artifact: artifact["entry"].encode("utf-8"))
    document_sha256 = consent[194:226].hex()
    if artifacts_digest(artifacts, "document") != document_sha256:
        raise SystemExit("the consent statement names another document digest")
    return {
        "profile": "qadra-evidence-manifest",
        "profile_version": 1,
        "purpose": "individual-document-signing",
        "deployment_id": ident(consent, 10),
        "case_id": ident(consent, 158),
        "document_id": ident(consent, 174),
        "document_version": int.from_bytes(consent[190:194], "big"),
        "document_sha256": document_sha256,
        "operation_id": ident(consent, 142),
        "algorithms": {
            "digest": "sha-256",
            "signature": "rsa-pkcs1v15-sha256",
            "timestamp": "rfc3161-sha256",
        },
        "artifacts": artifacts,
        "signers": [{
            "id": "signer-1",
            "account_id": ident(consent, 62),
            "binding_id": ident(consent, 94),
            "binding_registered_at": registered_at,
            "entries": {key: entries[role] for key, role in SIGNER_ENTRIES},
        }],
    }


def artifacts_digest(artifacts, role):
    return next(item["sha256"] for item in artifacts if item["role"] == role)


def canonical(value):
    text = json.dumps(value, sort_keys=True, separators=(",", ":"),
                      ensure_ascii=False)
    return text.encode("utf-8")


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--dir", required=True)
    parser.add_argument("--document-name", required=True)
    parser.add_argument("--binding-registered-at", required=True, type=int)
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    manifest = build(args.dir, args.document_name, args.binding_registered_at)
    with open(args.out, "wb") as handle:
        handle.write(canonical(manifest))


if __name__ == "__main__":
    main(sys.argv[1:])
