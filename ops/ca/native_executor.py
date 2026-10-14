"""Fixed native commands for public CA material and externally held CSR keys."""
import json
import os
from pathlib import Path
import selectors
import signal
import stat
import subprocess
import tempfile
import time
import uuid

import issuance_material as material


FAILURE = "CA executor failed; inspect the private operation receipt"
ADMISSION = ("csr_sha256", "spki_sha256", "root_fingerprint")
CERTIFICATE = ("serial", "fingerprint", "spki_sha256", "root_fingerprint", "purpose")
MAX_OUTPUT = 65536


def configured_path(value, *, directory=False):
    path = Path(value)
    if not path.is_absolute():
        raise ValueError("CA executor paths must be configured absolute paths")
    path = path.resolve(strict=True)
    if directory:
        accepted = path.is_dir()
    else:
        accepted = path.is_file() and os.access(path, os.X_OK)
    if not accepted:
        raise ValueError("CA executor configuration is unavailable")
    return path


def public_text(value):
    if not isinstance(value, bytes) or not 0 < len(value) <= 16384 or not value.isascii():
        raise ValueError("public CA material must be bounded ASCII bytes")
    return value.decode("ascii")


class NativeExecutor:
    def __init__(self, inspector_binary, cli_binary, scripts_dir, *, timeout=30):
        self.inspector = configured_path(inspector_binary)
        self.cli = configured_path(cli_binary)
        self.scripts = configured_path(scripts_dir, directory=True)
        if any(not (self.scripts / name).is_file() for name in ("issue-request.sh", "openssl.cnf")):
            raise ValueError("configured CA issuance scripts are unavailable")
        if type(timeout) not in (int, float) or not 0 < timeout <= 120:
            raise ValueError("CA executor timeout is invalid")
        self.timeout = timeout

    def _run(self, args, *, input_bytes=b"", pass_fds=(), env=None):
        """Bound stdout while draining it; terminate owned descendants on failure."""
        if not isinstance(input_bytes, bytes) or len(input_bytes) > 65536:
            raise ValueError("CA executor input exceeds its bound")
        process = None
        try:
            with tempfile.TemporaryFile() as source, selectors.DefaultSelector() as ready:
                source.write(input_bytes)
                source.seek(0)
                process = subprocess.Popen(args, stdin=source, stdout=subprocess.PIPE,
                    stderr=subprocess.DEVNULL, env=env, pass_fds=pass_fds,
                    start_new_session=True, umask=0o077)
                deadline = time.monotonic() + self.timeout
                output = bytearray()
                ready.register(process.stdout, selectors.EVENT_READ)
                while ready.get_map():
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        raise TimeoutError("native CA deadline reached")
                    for key, _ in ready.select(remaining):
                        chunk = os.read(key.fd, min(65536, MAX_OUTPUT + 1 - len(output)))
                        if not chunk:
                            ready.unregister(key.fileobj)
                        else:
                            output.extend(chunk)
                            if len(output) > MAX_OUTPUT:
                                raise ValueError("native CA output exceeds its bound")
                if process.wait(timeout=max(0.001, deadline - time.monotonic())) != 0:
                    raise RuntimeError(FAILURE)
                return bytes(output)
        except BaseException as error:
            if process is not None:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    # A surviving signer still owns its inherited deployment lock.
                    pass
            if isinstance(error, (KeyboardInterrupt, SystemExit)):
                raise
            raise RuntimeError(FAILURE) from None
        finally:
            if process is not None and process.stdout is not None:
                process.stdout.close()

    def _inspect(self, kind, value, root):
        payload = {"kind": kind, "material_pem": public_text(value), "root_pem": public_text(root)}
        encoded = json.dumps(payload, ensure_ascii=True).encode("ascii")
        if len(encoded) > 65536:
            raise ValueError("public inspection input exceeds its bound")
        raw = self._run([self.inspector, "inspect-public"], input_bytes=encoded)
        try:
            found = json.loads(raw)
            fields = (*ADMISSION, "subject") if kind == "request" else (
                *CERTIFICATE, "subject", "not_before", "not_after")
            result = {field: found[field] for field in fields}
            hashes = ADMISSION if kind == "request" else ("fingerprint", "spki_sha256", "root_fingerprint")
            if not all(material.hex_digest(result[field]) for field in hashes):
                raise ValueError("invalid public fingerprint")
            if (not isinstance(result["subject"], str) or not result["subject"]
                    or len(result["subject"].encode("utf-8")) > 16384):
                raise ValueError("invalid public subject")
            if kind == "request" and type(found["issuer_policy_matches"]) is not bool:
                raise ValueError("invalid issuer subject policy result")
            if kind != "request":
                purposes = ("partner", "document-signing", "time-stamping") if kind == "inventory-certificate" else ("partner", "document-signing")
                if (material.serial(result["serial"]) != result["serial"]
                        or result["purpose"] not in purposes
                        or type(result["not_before"]) is not int or type(result["not_after"]) is not int
                        or not 0 <= result["not_before"] < result["not_after"] <= 253402300799):
                    raise ValueError("invalid public certificate metadata")
        except (ValueError, TypeError, KeyError, UnicodeError):
            raise RuntimeError(FAILURE) from None
        if kind == "request" and not found["issuer_policy_matches"]:
            raise material.CsrSubjectPolicyMismatch({field: result[field] for field in ADMISSION})
        return result

    def describe_request(self, csr_pem, root_pem):
        return self._inspect("request", csr_pem, root_pem)

    def admit(self, csr_pem, root_pem):
        found = self.describe_request(csr_pem, root_pem)
        return {field: found[field] for field in ADMISSION}

    def describe_certificate(self, certificate_pem, root_pem):
        return self._inspect("inventory-certificate", certificate_pem, root_pem)

    def inspect_certificate(self, certificate_pem, root_pem):
        found = self._inspect("certificate", certificate_pem, root_pem)
        return {field: found[field] for field in CERTIFICATE}

    def issue(self, ca_dir, output_name, csr_pem, purpose, *, lock_fd):
        public_text(csr_pem)
        try:
            operation = uuid.UUID(output_name)
            if not operation.int or str(operation) != output_name:
                raise ValueError("invalid issuance identity")
        except (ValueError, TypeError, AttributeError):
            raise ValueError("issuance output must be a canonical non-nil UUID") from None
        if purpose not in ("partner", "document-signing"):
            raise ValueError("unsupported CA certificate purpose")
        ca = configured_path(ca_dir, directory=True)
        expected = (ca.parent.parent / "deploy.lock").lstat()
        held = os.fstat(lock_fd)
        if (not stat.S_ISREG(held.st_mode) or not stat.S_ISREG(expected.st_mode)
                or (held.st_dev, held.st_ino) != (expected.st_dev, expected.st_ino)):
            raise ValueError("issuance must retain the shared deployment lock")
        with tempfile.TemporaryDirectory(prefix="qadra-ca-csr-") as temporary:
            path = Path(temporary) / "request.pem"
            descriptor = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
            with os.fdopen(descriptor, "wb") as stream:
                stream.write(csr_pem)
                stream.flush()
                os.fsync(stream.fileno())
            env = {**os.environ, "PKI_CA_DIR": str(ca), "LC_ALL": "C", "TZ": "UTC"}
            self._run([self.cli, "--json", "pki", "--scripts-dir", self.scripts, "issue-request",
                       "--csr", path, "--name", output_name, "--purpose", purpose],
                      pass_fds=(lock_fd,), env=env)

    def revoke(self, ca_dir, certificate_pem, *, lock_fd):
        """Only revoke locally; the configured script never generates or publishes CRL."""
        public_text(certificate_pem)
        ca = configured_path(ca_dir, directory=True)
        script = self.scripts / "revoke.sh"
        if not script.is_file():
            raise ValueError("configured CA revocation script is unavailable")
        expected = (ca.parent.parent / "deploy.lock").lstat()
        held = os.fstat(lock_fd)
        if (not stat.S_ISREG(held.st_mode) or not stat.S_ISREG(expected.st_mode)
                or (held.st_dev, held.st_ino) != (expected.st_dev, expected.st_ino)):
            raise ValueError("revocation must retain the shared deployment lock")
        with tempfile.TemporaryDirectory(prefix="qadra-ca-revoke-") as temporary:
            path = Path(temporary) / "certificate.pem"
            descriptor = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
            with os.fdopen(descriptor, "wb") as stream:
                stream.write(certificate_pem)
                stream.flush()
                os.fsync(stream.fileno())
            env = {**os.environ, "PKI_CA_DIR": str(ca), "LC_ALL": "C", "TZ": "UTC"}
            self._run(["/bin/bash", script, path], pass_fds=(lock_fd,), env=env)
