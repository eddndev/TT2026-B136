"""Provision isolated databases and initial cryptographic state once."""
import json
from pathlib import Path
import subprocess
import time

from runtime import environment, run, settings


def create_databases(root):
    config = settings(root)
    directory = root / "data/postgres"
    postgres_bin = subprocess.check_output(["pg_config", "--bindir"], text=True).strip()
    if not (directory / "PG_VERSION").exists():
        password = root / "config/initdb-password"
        password.write_text(config["admin_password"] + "\n")
        try:
            run([Path(postgres_bin) / "initdb", "-D", directory, "--username=qadra_admin",
                 "--auth=scram-sha-256", "--encoding=UTF8", "--no-locale", f"--pwfile={password}"])
        finally:
            password.unlink(missing_ok=True)
        with (directory / "postgresql.conf").open("a") as stream:
            stream.write(f"\nlisten_addresses='127.0.0.1'\nport={config['postgres_port']}\n"
                         f"unix_socket_directories='{root}/run/pg'\nmax_connections=50\nshared_buffers=128MB\n")
    run(["systemctl", "--user", "enable", "--now", "qadra-postgres.service", "qadra-redis.service"])
    env = environment(root, admin=True)
    env["PGDATABASE"] = "postgres"
    for _ in range(30):
        result = subprocess.run(["psql", "-XAt", "-c", "SELECT 1"], env=env,
                                capture_output=True, text=True)
        if result.returncode == 0:
            break
        time.sleep(1)
    else:
        raise RuntimeError("PostgreSQL did not become ready")
    # Generated hexadecimal passwords cannot contain SQL quoting characters.
    sql = rf"""SELECT 'CREATE ROLE qadra_runtime LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT PASSWORD ''{config['runtime_password']}'''
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname='qadra_runtime')\gexec
SELECT 'CREATE DATABASE qadra ENCODING ''UTF8'''
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname='qadra')\gexec
"""
    run(["psql", "-X", "-v", "ON_ERROR_STOP=1", "-q"], env=env, input=sql, text=True,
        stdout=subprocess.DEVNULL)
    print("Isolated PostgreSQL and Redis ready on loopback")


def initial_trust_is_published(root, env):
    query = """SELECT row_to_json(trust) FROM (
        SELECT r.revision, encode(a.root_der, 'hex') AS root_der,
               encode(r.crl_der, 'hex') AS crl_der,
               floor(extract(epoch FROM CURRENT_TIMESTAMP))
                   BETWEEN r.valid_from AND r.valid_until AS valid_now
        FROM participant_credential_authority a
        JOIN participant_credential_trust_revisions r USING (deployment_id)
        ORDER BY r.revision DESC LIMIT 1
    ) AS trust"""
    result = run(["psql", "-XAt", "-v", "ON_ERROR_STOP=1", "-c", query],
                 env=env, capture_output=True, text=True, timeout=15)
    if not result.stdout.strip():
        return False
    try:
        trust = json.loads(result.stdout)
        if (not isinstance(trust, dict)
                or set(trust) != {"revision", "root_der", "crl_der", "valid_now"}
                or type(trust["revision"]) is not int or trust["revision"] != 1
                or trust["valid_now"] is not True):
            raise ValueError("unexpected initial trust state")
        for field, command, path, maximum in (
                ("root_der", "x509", root / "data/ca/ca.crt.pem", 16384),
                ("crl_der", "crl", root / "data/ca/crl/crl.pem", 1048576)):
            stored = bytes.fromhex(trust[field])
            if not stored or len(stored) > maximum:
                raise ValueError("invalid initial trust material")
            local = run(["openssl", command, "-in", path, "-outform", "DER"],
                        capture_output=True, timeout=10).stdout
            if local != stored:
                raise ValueError("initial trust material differs")
    except (ValueError, TypeError, KeyError) as error:
        raise RuntimeError("initial credential trust differs or requires maintenance") from error
    return True


def initialize(root, target):
    marker = root / "config/schema"
    if marker.exists():
        return
    env = environment(root, admin=True)
    env["LD_LIBRARY_PATH"] = str(target / "lib")
    binary = target / "bin/despacho-cli"
    run([binary, "database", "migrate", "--runtime-role", "qadra_runtime"], env=env,
        cwd=root / "data", timeout=300)
    # Reconcile committed trust before creating any potentially missing PKI files.
    published = initial_trust_is_published(root, env)
    ca = root / "data/ca"
    tsa = root / "data/tsa"
    scripts = target / "pki"
    if published:
        required = (ca / "private/ca.key.pem", ca / "certs/qadra-server.crt.pem",
                    ca / "private/qadra-server.key.pem", tsa / "tsa.crt.pem",
                    tsa / "private/tsa.key.pem", tsa / "tsa-chain.pem", tsa / "serial")
        if any(not path.is_file() or path.stat().st_size == 0 for path in required):
            raise RuntimeError("published trust requires complete existing PKI state; restore it before retrying")
    if not (ca / "ca.crt.pem").exists():
        run(["bash", scripts / "init-ca.sh"], env=env, cwd=root / "data")
    if not (ca / "certs/qadra-server.crt.pem").exists():
        run(["bash", scripts / "issue-cert.sh", "Qadra Server"], env=env, cwd=root / "data")
    if not (tsa / "tsa.crt.pem").exists():
        run(["bash", scripts / "issue-tsa-cert.sh"], env=env, cwd=root / "data")
    if not (ca / "crl/crl.pem").exists():
        run(["bash", scripts / "gen-crl.sh"], env=env, cwd=root / "data")
    # A committed initial publication can outlive an interrupted marker write.
    if not published:
        run([binary, "credential-trust", "publish", "--root-cert", ca / "ca.crt.pem",
             "--crl", ca / "crl/crl.pem", "--expected-revision", "0"],
            env=env, cwd=root / "data")
    marker.write_text(json.loads((target / "release.json").read_text())["schema"] + "\n")
