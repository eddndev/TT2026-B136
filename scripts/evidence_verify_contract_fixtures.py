#!/usr/bin/env python3
"""Statements, manifest and records of the bilateral contract fixtures.

Called by scripts/evidence-verify-contract-fixtures.sh with python3 -I. It
uses only the Python standard library.

Every command reads and writes package entries in the --dir directory under
their exact package names, and keeps the identifiers and instants of the
fixture request in the --state JSON file:

- "document" writes contrato.pdf, a PDF-shaped file larger than 64 KiB;
- "begin" first rebuilds the shared statement vector to prove that this
  encoder still matches it, then writes contrato-solicitud.bin (CTRTREQ1),
  both certificate bindings (DOCBIND1) and the position 1 statement
  (CTRTSIG1); for the variant ceremonies --request-id replaces the request
  identity and --firm-as-client makes the position 2 binding and statement
  name the Client account;
- "counter" writes the position 2 statement, which carries the SHA-256 of
  the position 1 signature and of its time-stamp response;
- "manifest" writes contrato-manifiesto.json over every covered entry;
- "records" writes contrato-recibo.json and contrato-acuse-parcial.json;
- "index" writes fixtures.json with the digest and size of every file,
  variant files included under their relative paths, and the instants the
  generator recorded.

The JSON outputs are the RFC 8785 form of the admitted subset: keys sorted,
no whitespace, ASCII strings and integers only, no trailing newline.

Rationale: docs/adr/0086-bilateral-contract-signing.md.
"""

import argparse
import hashlib
import json
import os
import struct
import sys
import uuid

VECTOR_FIELDS = (
    "deployment_id", "request_id", "case_id", "document_id",
    "client_account_id", "firm_account_id", "requester_account_id",
)
REQUEST = {
    "deployment_id": "7a1c2a3b-4d5e-4f60-8a71-b2c3d4e5f601",
    "request_id": "7a1c2a3b-4d5e-4f60-8a71-b2c3d4e5f602",
    "case_id": "7a1c2a3b-4d5e-4f60-8a71-b2c3d4e5f603",
    "document_id": "7a1c2a3b-4d5e-4f60-8a71-b2c3d4e5f604",
    "client_account_id": "7a1c2a3b-4d5e-4f60-8a71-b2c3d4e5f605",
    "firm_account_id": "7a1c2a3b-4d5e-4f60-8a71-b2c3d4e5f606",
    "requester_account_id": "7a1c2a3b-4d5e-4f60-8a71-b2c3d4e5f607",
    "document_version": 2,
}
PARTIES = (
    {
        "account": "client_account_id",
        "binding_id": "7a1c2a3b-4d5e-4f60-8a71-b2c3d4e5f611",
        "operation_id": "7a1c2a3b-4d5e-4f60-8a71-b2c3d4e5f621",
        "account_revision": 4,
        "authentication_generation": 2,
        "trust_revision": 3,
    },
    {
        "account": "firm_account_id",
        "binding_id": "7a1c2a3b-4d5e-4f60-8a71-b2c3d4e5f612",
        "operation_id": "7a1c2a3b-4d5e-4f60-8a71-b2c3d4e5f622",
        "account_revision": 9,
        "authentication_generation": 5,
        "trust_revision": 3,
    },
)
CEREMONY_REFERENCE = "HOJA-PRUEBA-0001"
DOCUMENT_SIZE = 70 * 1024

# Role, party and entry of every artifact the manifest covers, as the
# table of docs/adr/0086-bilateral-contract-signing.md fixes them.
SHARED = (
    ("document", "contrato.pdf"),
    ("document-timestamp", "contrato.pdf.tsr"),
    ("request", "contrato-solicitud.bin"),
    ("issuer-certificate", "ca.pem"),
    ("tsa-chain", "tsa-chain.pem"),
)
PER_PARTY = (
    ("statement", "statement", "declaracion.bin"),
    ("statement-signature", "statement_signature", "declaracion.sig"),
    ("signer-certificate", "signer_certificate", "certificado.pem"),
    ("issuer-crl", "issuer_crl", "crl.pem"),
    ("binding-statement", "binding_statement", "vinculo.bin"),
    ("binding-signature", "binding_signature", "vinculo.sig"),
    ("binding-root", "binding_root", "vinculo-raiz.der"),
    ("binding-crl", "binding_crl", "vinculo-crl.der"),
)
FIRST_PARTY = (
    ("statement-timestamp", "parte-1-declaracion.sig.tsr"),
    ("statement-tsa-chain", "parte-1-tsa-chain.pem"),
)
ALGORITHMS = {
    "digest": "sha-256",
    "signature": "rsa-pkcs1v15-sha256",
    "timestamp": "rfc3161-sha256",
}


def sha256(data):
    return hashlib.sha256(data).digest()


def read(path):
    with open(path, "rb") as handle:
        return handle.read()


def write(path, data):
    with open(path, "wb") as handle:
        handle.write(data)


def canonical(value):
    text = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
    return text.encode("ascii")


def ident(text):
    return uuid.UUID(text).bytes


def request_bytes(values):
    data = (
        b"CTRTREQ1" + bytes([1, 1])
        + b"".join(ident(values[key]) for key in VECTOR_FIELDS[:4])
        + struct.pack(">I", values["document_version"])
        + bytes.fromhex(values["document_sha256"])
        + b"".join(ident(values[key]) for key in VECTOR_FIELDS[4:])
        + bytes.fromhex(values["client_certificate_sha256"])
        + bytes.fromhex(values["firm_certificate_sha256"])
        + struct.pack(">q", values["created_at_seconds"])
        + struct.pack(">q", values["expires_at_seconds"])
        + bytes([2, 1])
    )
    assert len(data) == 240
    return data


def prefix(magic, deployment, party):
    data = (
        magic + bytes([1, 1]) + ident(deployment)
        + bytes.fromhex(party["root_sha256"])
        + struct.pack(">I", party["trust_revision"])
        + ident(party["account_id"])
        + struct.pack(">Q", party["account_revision"])
        + struct.pack(">Q", party["authentication_generation"])
        + ident(party["binding_id"])
        + bytes.fromhex(party["certificate_sha256"])
    )
    assert len(data) == 142
    return data


def statement_bytes(party, request, values):
    data = (
        prefix(b"CTRTSIG1", values["deployment_id"], party)
        + ident(party["operation_id"]) + ident(values["request_id"])
        + sha256(request) + bytes.fromhex(values["document_sha256"])
        + bytes([party["position"]])
        + bytes.fromhex(party["prior_signature_sha256"])
        + bytes.fromhex(party["prior_token_sha256"])
    )
    assert len(data) == 303
    return data


def check_vectors(path):
    vectors = json.loads(read(path))
    values = vectors["request_inputs"]
    request = request_bytes(values)
    if request.hex() != vectors["request"]["hex"]:
        raise SystemExit("CTRTREQ1 encoder differs from the shared vector")
    for key, expected in (("client_party", "client_statement"),
                          ("firm_party", "firm_statement")):
        if statement_bytes(vectors[key], request, values).hex() != vectors[expected]["hex"]:
            raise SystemExit("CTRTSIG1 encoder differs from the shared vector")


def load(args):
    with open(args.state, encoding="ascii") as handle:
        return json.load(handle)


def save(args, state):
    with open(args.state, "w", encoding="ascii") as handle:
        json.dump(state, handle, indent=2, sort_keys=True)


def entry(args, name):
    return os.path.join(args.dir, name)


def command_document(args):
    head = b"%PDF-1.7\n% evidence-verify bilateral contract fixture\n"
    tail = b"%%EOF\n"
    line = b"% contract clause padding that keeps the document over 64 KiB\n"
    body = (line * (DOCUMENT_SIZE // len(line) + 1))[: DOCUMENT_SIZE - len(head) - len(tail)]
    write(entry(args, "contrato.pdf"), head + body + tail)


def party_values(index, root_sha256, leaf_sha256, registered_at):
    party = dict(PARTIES[index])
    party.update({
        "position": index + 1,
        "account_id": REQUEST[party.pop("account")],
        "root_sha256": root_sha256,
        "certificate_sha256": leaf_sha256,
        "binding_registered_at": registered_at,
        "prior_signature_sha256": "00" * 32,
        "prior_token_sha256": "00" * 32,
    })
    return party


def command_begin(args):
    check_vectors(args.vectors)
    root = sha256(read(args.root_der)).hex()
    leaves = [sha256(read(path)).hex() for path in (args.client_der, args.firm_der)]
    values = dict(REQUEST)
    if args.request_id:
        values["request_id"] = args.request_id
    values.update({
        "document_sha256": sha256(read(entry(args, "contrato.pdf"))).hex(),
        "client_certificate_sha256": leaves[0],
        "firm_certificate_sha256": leaves[1],
        "created_at_seconds": args.created_at,
        "expires_at_seconds": args.expires_at,
    })
    request = request_bytes(values)
    write(entry(args, "contrato-solicitud.bin"), request)
    registered = (args.client_registered_at, args.firm_registered_at)
    parties = [party_values(index, root, leaves[index], registered[index]) for index in (0, 1)]
    if args.firm_as_client:
        parties[1]["account_id"] = values["client_account_id"]
    for party in parties:
        name = f"parte-{party['position']}-vinculo.bin"
        write(entry(args, name), prefix(b"DOCBIND1", values["deployment_id"], party))
    write(entry(args, "parte-1-declaracion.bin"), statement_bytes(parties[0], request, values))
    save(args, {"request": values, "parties": parties})


def command_counter(args):
    state = load(args)
    party = state["parties"][1]
    party["prior_signature_sha256"] = sha256(read(entry(args, "parte-1-declaracion.sig"))).hex()
    party["prior_token_sha256"] = sha256(read(entry(args, "parte-1-declaracion.sig.tsr"))).hex()
    request = read(entry(args, "contrato-solicitud.bin"))
    write(entry(args, "parte-2-declaracion.bin"),
          statement_bytes(party, request, state["request"]))
    save(args, state)


def artifact(args, party, role, name):
    data = read(entry(args, name))
    return {"entry": name, "party": party, "role": role,
            "sha256": sha256(data).hex(), "size": len(data)}


def command_manifest(args):
    state = load(args)
    values = state["request"]
    rows = [artifact(args, 0, role, name) for role, name in SHARED]
    rows += [artifact(args, 1, role, name) for role, name in FIRST_PARTY]
    signers = []
    for index, party in enumerate(state["parties"]):
        position = index + 1
        entries = {}
        for role, member, suffix in PER_PARTY:
            name = f"parte-{position}-{suffix}"
            rows.append(artifact(args, position, role, name))
            entries[member] = name
        signers.append({
            "id": f"signer-{position}",
            "position": position,
            "side": ("client", "firm")[index],
            "account_id": party["account_id"],
            "account_role": ("client", "litigator")[index],
            "binding_id": party["binding_id"],
            "binding_registered_at": party["binding_registered_at"],
            "operation_id": party["operation_id"],
            "entries": entries,
        })
    rows.sort(key=lambda row: row["entry"].encode("ascii"))
    manifest = {
        "profile": "qadra-evidence-manifest",
        "profile_version": 2,
        "purpose": "bilateral-contract-signing",
        "deployment_id": values["deployment_id"],
        "case_id": values["case_id"],
        "request_id": values["request_id"],
        "document_id": values["document_id"],
        "document_version": values["document_version"],
        "document_sha256": values["document_sha256"],
        "request_sha256": sha256(read(entry(args, "contrato-solicitud.bin"))).hex(),
        "expires_at": values["expires_at_seconds"],
        "algorithms": ALGORITHMS,
        "artifacts": rows,
        "signers": signers,
    }
    write(entry(args, "contrato-manifiesto.json"), canonical(manifest))


def command_records(args):
    state = load(args)
    request_id = state["request"]["request_id"]
    request_sha256 = sha256(read(entry(args, "contrato-solicitud.bin"))).hex()
    common = {"provenance": "application-record", "request_id": request_id,
              "request_sha256": request_sha256, "ceremony_reference": CEREMONY_REFERENCE}
    receipt = dict(common, client_accepted_at=args.client_accepted_at,
                   firm_accepted_at=args.firm_accepted_at, completed_at=args.completed_at)
    partial = dict(common, accepted_at=args.client_accepted_at,
                   binding_registered_at=state["parties"][0]["binding_registered_at"],
                   exported_at=args.exported_at)
    write(entry(args, "contrato-recibo.json"), canonical(receipt))
    write(entry(args, "contrato-acuse-parcial.json"), canonical(partial))


def command_index(args):
    files = {}
    for folder, _, names in os.walk(args.dir):
        for name in names:
            path = os.path.relpath(os.path.join(folder, name), args.dir).replace(os.sep, "/")
            if path in ("fixtures.json", "README.md"):
                continue
            data = read(os.path.join(folder, name))
            files[path] = {"sha256": sha256(data).hex(), "size": len(data)}
    state = load(args)
    index = {
        "generated_at": args.generated_at,
        "instants": {"t1": args.t1, "td": args.td, "tf": args.tf,
                     "renewed_crl_this_update": args.renewed_this_update},
        "request": state["request"],
        "files": files,
    }
    with open(entry(args, "fixtures.json"), "w", encoding="ascii") as handle:
        json.dump(index, handle, indent=2, sort_keys=True)
        handle.write("\n")


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--dir", required=True)
    parser.add_argument("--state", required=True)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("document")
    begin = commands.add_parser("begin")
    for name in ("vectors", "root-der", "client-der", "firm-der"):
        begin.add_argument(f"--{name}", required=True)
    for name in ("created-at", "expires-at", "client-registered-at", "firm-registered-at"):
        begin.add_argument(f"--{name}", required=True, type=int)
    begin.add_argument("--request-id")
    begin.add_argument("--firm-as-client", action="store_true")
    commands.add_parser("counter")
    commands.add_parser("manifest")
    records = commands.add_parser("records")
    for name in ("client-accepted-at", "firm-accepted-at", "completed-at", "exported-at"):
        records.add_argument(f"--{name}", required=True, type=int)
    index = commands.add_parser("index")
    for name in ("generated-at", "t1", "td", "tf", "renewed-this-update"):
        index.add_argument(f"--{name}", required=True, type=int)
    args = parser.parse_args(argv)
    handlers = {
        "document": command_document, "begin": command_begin, "counter": command_counter,
        "manifest": command_manifest, "records": command_records, "index": command_index,
    }
    handlers[args.command](args)


if __name__ == "__main__":
    main(sys.argv[1:])
