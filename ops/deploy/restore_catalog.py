"""Fixed read-only catalog and audit queries for the private restore collector."""
import json
import re

import backup_manifest_schema
import restore_admission_schema as schema
from restore_commands import run


def read(tools, environment, query, *, maximum=65536):
    sql = ("BEGIN READ ONLY; SET LOCAL search_path=pg_catalog,pg_temp; "
           "SET LOCAL statement_timeout='5s'; SET LOCAL lock_timeout='1s'; " + query + "; COMMIT;")
    return run([tools["psql"], "-XqAt", "-v", "ON_ERROR_STOP=1", "-c", sql],
               env=environment, timeout=10, maximum=maximum).stdout


def catalog(tools, environment, target):
    for key in ("database", "schema", "owner", "runtime_role"):
        schema.identifier(target[key])
    owner, runtime, namespace = (target[key] for key in ("owner", "runtime_role", "schema"))
    query = f"""WITH database AS (
      SELECT * FROM pg_catalog.pg_database WHERE datname=pg_catalog.current_database()
    ), roles AS (
      SELECT * FROM pg_catalog.pg_roles WHERE rolname IN ('{owner}','{runtime}')
    ) SELECT pg_catalog.json_build_object(
      'database', d.datname, 'schema', '{namespace}',
      'database_owner', pg_catalog.pg_get_userbyid(d.datdba),
      'schema_owner', (SELECT pg_catalog.pg_get_userbyid(nspowner)
                      FROM pg_catalog.pg_namespace WHERE nspname='{namespace}'),
      'server_version_num', pg_catalog.current_setting('server_version_num')::pg_catalog.int4,
      'data_directory', pg_catalog.current_setting('data_directory'),
      'encoding', pg_catalog.pg_encoding_to_char(d.encoding),
      'collate', d.datcollate, 'ctype', d.datctype, 'locale_provider', d.datlocprovider,
      'roles', (SELECT pg_catalog.json_object_agg(r.rolname, pg_catalog.json_build_object(
        'login', r.rolcanlogin, 'superuser', r.rolsuper, 'createdb', r.rolcreatedb,
        'createrole', r.rolcreaterole, 'inherit', r.rolinherit,
        'replication', r.rolreplication, 'bypassrls', r.rolbypassrls,
        'connection_limit', r.rolconnlimit, 'member_of', (SELECT COALESCE(
          pg_catalog.json_agg(p.rolname ORDER BY p.rolname), '[]'::pg_catalog.json)
          FROM pg_catalog.pg_auth_members m JOIN pg_catalog.pg_roles p ON p.oid=m.roleid
          WHERE m.member=r.oid))) FROM roles r),
      'role_settings', (SELECT COALESCE(pg_catalog.json_agg(r.rolname), '[]'::pg_catalog.json)
        FROM roles r WHERE r.rolconfig IS NOT NULL),
      'role_expirations', (SELECT COALESCE(pg_catalog.json_agg(r.rolname), '[]'::pg_catalog.json)
        FROM roles r WHERE r.rolvaliduntil IS NOT NULL),
      'database_settings', (SELECT COALESCE(pg_catalog.json_agg(s), '[]'::pg_catalog.json)
        FROM pg_catalog.pg_db_role_setting s WHERE s.setdatabase=d.oid
        OR s.setrole IN (SELECT oid FROM roles) OR (s.setdatabase=0 AND s.setrole=0)),
      'memberships', (SELECT COALESCE(pg_catalog.json_agg(m), '[]'::pg_catalog.json)
        FROM pg_catalog.pg_auth_members m WHERE m.member IN (SELECT oid FROM roles)),
      'other_clients', (SELECT COALESCE(pg_catalog.json_agg(a.pid), '[]'::pg_catalog.json)
        FROM pg_catalog.pg_stat_activity a WHERE a.datname=d.datname
        AND a.pid<>pg_catalog.pg_backend_pid() AND a.backend_type='client backend')
    ) FROM database d"""
    raw = read(tools, environment, query)
    if len(raw) > 65536:
        raise ValueError("restore catalog exceeds its byte boundary")
    value = backup_manifest_schema.decode(raw)
    fields = ("database", "schema", "database_owner", "schema_owner", "server_version_num",
              "data_directory", "encoding", "collate", "ctype", "locale_provider", "roles",
              "role_settings", "role_expirations", "database_settings", "memberships", "other_clients")
    backup_manifest_schema.keys(value, fields)
    return value


def audit(tools, environment, namespace):
    schema.identifier(namespace)
    query = f"""SELECT pg_catalog.json_build_object(
      'sequence', sequence, 'timestamp', timestamp, 'actor', actor,
      'action', action, 'resource', resource, 'chain', pg_catalog.encode(chain,'hex'))
      FROM "{namespace}".audit_events ORDER BY sequence LIMIT 100001"""
    raw = read(tools, environment, query, maximum=8 * 1024 * 1024)
    lines = raw.splitlines()
    if len(raw) > 8 * 1024 * 1024 or len(lines) > 100000:
        raise ValueError("restore audit exceeds its boundary")
    rows = []
    for sequence, line in enumerate(lines):
        row = backup_manifest_schema.decode(line)
        backup_manifest_schema.keys(row, ("sequence", "timestamp", "actor", "action", "resource", "chain"))
        if type(row["sequence"]) is not int or row["sequence"] != sequence:
            raise ValueError("restore audit sequence is incomplete")
        timestamp = row["timestamp"]
        if (not isinstance(timestamp, str) or not 20 <= len(timestamp) <= 30
                or re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}"
                                r"(?:\.[0-9]{0,8}[1-9])?Z", timestamp) is None):
            raise ValueError("restore audit timestamp is not canonical UTC text")
        # Preserve nanoseconds and canonical bytes for the final CLI verifier.
        backup_manifest_schema.hexadecimal(row["chain"], 64)
        if any(not isinstance(row[key], str) for key in ("actor", "action", "resource")):
            raise ValueError("restore audit event has invalid text")
        rows.append(row)
    encoded = b"".join((json.dumps(row, separators=(",", ":")) + "\n").encode("utf-8") for row in rows)
    if len(encoded) > 8 * 1024 * 1024:
        raise ValueError("restore audit serialization exceeds its boundary")
    return rows, encoded
