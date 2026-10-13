"""Observe the CA root and the CRL revision actually persisted by Qadra."""
import base64
from pathlib import Path
import tempfile
import time

import crl_journal as journal
import crl_material
import inventory
import issuance_material as material


def pem(der_hex, kind):
    encoded = base64.b64encode(bytes.fromhex(der_hex)).decode("ascii")
    return ("-----BEGIN " + kind + "-----\n" +
            "\n".join(encoded[index:index + 64] for index in range(0, len(encoded), 64)) +
            "\n-----END " + kind + "-----\n")


def public_state(root, expected_root_fingerprint):
    root = Path(root).resolve(strict=True)
    if not material.hex_digest(expected_root_fingerprint):
        raise ValueError("the adopted CA fingerprint is invalid")
    with inventory._read_locked(root):
        head = crl_material.read_head(root)
        if head["root_fingerprint"] != expected_root_fingerprint:
            raise ValueError("persisted CA differs from the adopted authority")
        # Inspect persisted DER, not a possibly newer local CRL file. These are
        # public temporary inputs to the existing CRL verifier, never CA keys.
        with tempfile.TemporaryDirectory(prefix="ca-public-trust-") as name:
            directory = Path(name)
            cert = directory / "root.pem"
            crl = directory / "crl.pem"
            cert.write_text(pem(head["root_der"], "CERTIFICATE"), encoding="ascii")
            crl.write_text(pem(head["crl_der"], "X509 CRL"), encoding="ascii")
            found = crl_material.inspect(cert, crl)
            if any(found[key] != head[key] for key in crl_material.FIELDS):
                raise ValueError("persisted public trust cannot be reconciled")
            local_root = material.read(root / "data/ca/ca.crt.pem", 16384)
            cert.write_bytes(local_root)
            if crl_material.inspect(cert, crl)["root_fingerprint"] != expected_root_fingerprint:
                raise ValueError("local root differs from the adopted CA")
        now = int(time.time())
        return {**{key: head[key] for key in ("deployment_id", "revision", "root_fingerprint",
                "crl_digest", "crl_number", "valid_from", "valid_until")},
                "checked_at": now, "valid_now": head["valid_from"] <= now <= head["valid_until"],
                "maintenance_pending": journal.pending(root),
                "revoked_serials": found["revoked_serials"],
                "root_pem": pem(head["root_der"], "CERTIFICATE"),
                "crl_pem": pem(head["crl_der"], "X509 CRL")}
