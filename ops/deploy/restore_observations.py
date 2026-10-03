"""Read exact local engine, role and tool facts without changing backend state."""
import os
from pathlib import Path
import re
import stat

import backup_manifest_schema as capture
from backup_manifest_files import inspect
import restore_admission_schema as schema
import restore_catalog
from restore_commands import run
from runtime import environment, settings


TOOL_NAMES = ("postgres", "psql", "pg_dump", "pg_restore", "redis_server", "redis_cli", "redis_check_rdb")


def private_directory(path):
    if (not isinstance(path, Path) or not path.is_absolute() or path.resolve(strict=True) != path
            or any(parent.is_symlink() for parent in (path, *path.parents))
            or not stat.S_ISDIR(path.stat().st_mode) or stat.S_IMODE(path.stat().st_mode) != 0o700):
        raise ValueError("restore observation requires an exact private directory")


def executable(path):
    if (not isinstance(path, Path) or not path.is_absolute() or path.resolve(strict=True) != path
            or any(parent.is_symlink() for parent in (path, *path.parents))
            or not path.is_file() or not os.access(path, os.X_OK)):
        raise ValueError("restore tool must be an exact executable file")
    return inspect(path, 512 * 1024 * 1024)[0]


def process(pid, tool):
    if type(pid) is not int or not 0 < pid < 2**31:
        raise ValueError("restore process identity is invalid")
    if Path(f"/proc/{pid}/exe").resolve(strict=True) != tool:
        raise ValueError("restore process executable differs from the selected tool")


def inputs(root, targets, tools):
    private_directory(root)
    capture.keys(targets, ("postgres", "redis"))
    capture.keys(tools, TOOL_NAMES)
    capture.keys(targets["postgres"], ("pid", "data_directory", "database", "schema", "owner", "runtime_role"))
    capture.keys(targets["redis"], ("pid", "directory", "database"))
    pg, redis = targets["postgres"], targets["redis"]
    if (pg["database"], pg["owner"], pg["runtime_role"]) != ("qadra", "qadra_admin", "qadra_runtime"):
        raise ValueError("restore collector requires the provisioned target profile")
    schema.identifier(pg["schema"])
    schema.integer(redis["database"], 0, 0)
    private_directory(pg["data_directory"])
    private_directory(redis["directory"])
    if redis["directory"] != root / "data":
        raise ValueError("restore Redis directory differs from the managed target")
    records = {name: executable(path) for name, path in tools.items()}
    process(pg["pid"], tools["postgres"])
    process(redis["pid"], tools["redis_server"])
    _, pidfile = inspect(pg["data_directory"] / "postmaster.pid", 4096, private=True, contents=True)
    lines = pidfile.decode("ascii").splitlines()
    if len(lines) < 2 or lines[:2] != [str(pg["pid"]), str(pg["data_directory"])]:
        raise ValueError("restore PostgreSQL directory differs from the owned process")
    return records


def version(tool, environment, expression):
    output = run([tool, "--version"], env=environment, timeout=5, maximum=8192, text=True).stdout
    found = re.fullmatch(expression, output.strip()) if len(output.encode("utf-8")) <= 8192 else None
    if found is None:
        raise ValueError("restore tool reported an unsupported version")
    return found.group(1)


def redis_read(tools, config, *arguments):
    result = run([tools["redis_cli"], "-h", "127.0.0.1", "-p", str(config["redis_port"]),
                  "-n", "0", "--raw", *arguments],
                 env={**os.environ, "REDISCLI_AUTH": config["redis_password"]},
                 timeout=10, maximum=65536, text=True).stdout
    if len(result.encode("utf-8")) > 65536:
        raise ValueError("restore Redis observation exceeds its boundary")
    return result


def info(raw):
    value = {}
    for line in raw.splitlines():
        if not line or line.startswith("#"):
            continue
        key, separator, entry = line.partition(":")
        if not separator or key in value:
            raise ValueError("restore Redis observation is ambiguous")
        value[key] = entry
    return value


def observe(root, targets, tools):
    records = inputs(root, targets, tools)
    config = settings(root)
    for field in ("postgres_port", "redis_port"):
        schema.integer(config[field], 1, 65535)
    env = environment(root, admin=True)
    pg_target = targets["postgres"]
    catalog = restore_catalog.catalog(tools, env, pg_target)
    effective_owner = catalog["schema_owner"]
    if effective_owner == "pg_database_owner":
        effective_owner = catalog["database_owner"]
    if (catalog["database"] != pg_target["database"] or catalog["schema"] != pg_target["schema"]
            or catalog["database_owner"] != pg_target["owner"] or effective_owner != pg_target["owner"]
            or catalog["data_directory"] != str(pg_target["data_directory"])
            or catalog["encoding"] != "UTF8" or catalog["collate"] != "C" or catalog["ctype"] != "C"
            or catalog["locale_provider"] != "c"
            or any(catalog[key] != [] for key in ("role_settings", "role_expirations", "database_settings",
                                                 "memberships", "other_clients"))):
        raise ValueError("restore PostgreSQL observations differ from the closed supported profile")
    schema.integer(catalog["server_version_num"], 160000, 169999)
    pg = {name: catalog[name] for name in ("database", "schema", "server_version_num", "encoding", "collate", "ctype", "roles")}
    pg.update(owner=pg_target["owner"], runtime_role=pg_target["runtime_role"], dump_format="custom", locale_provider="libc")
    for name in ("pg_dump", "pg_restore"):
        pg[name + "_version"] = version(tools[name], env, name + r" \(PostgreSQL\) (16\.[0-9]+)(?: [^\r\n]*)?")
    pg["tools"] = {name: records[tool] for name, tool in
                   (("server", "postgres"), ("pg_dump", "pg_dump"), ("pg_restore", "pg_restore"))}
    schema.postgres(pg)
    if redis_read(tools, config, "PING").strip() != "PONG":
        raise ValueError("restore Redis is unavailable")
    server = info(redis_read(tools, config, "INFO", "server"))
    if server.get("process_id") != str(targets["redis"]["pid"]):
        raise ValueError("restore Redis process differs from its owned target")
    valkey = server.get("valkey_version")
    if valkey:
        if server.get("server_name", "valkey") != "valkey":
            raise ValueError("restore Redis engine identity is contradictory")
        engine, release = "valkey", valkey
    else:
        if server.get("server_name", "redis") != "redis":
            raise ValueError("restore Redis engine identity is unsupported")
        engine, release = "redis", server.get("redis_version")
    schema.version(release, 3)
    fields = redis_read(tools, config, "CONFIG", "GET", "dir", "dbfilename", "appendonly", "appenddirname").splitlines()
    if len(fields) != 8 or len(set(fields[::2])) != 4:
        raise ValueError("restore Redis persistence configuration is ambiguous")
    configured = dict(zip(fields[::2], fields[1::2]))
    if (set(configured) != {"dir", "dbfilename", "appendonly", "appenddirname"}
            or configured["dir"] != str(targets["redis"]["directory"])
            or configured["dbfilename"] != "redis.rdb" or configured["appenddirname"] != "appendonlydir"
            or configured["appendonly"] not in ("yes", "no")):
        raise ValueError("restore Redis persistence differs from its private profile")
    persistence = info(redis_read(tools, config, "INFO", "persistence"))
    enabled = configured["appendonly"] == "yes"
    if (persistence.get("aof_enabled") != str(int(enabled))
            or persistence.get("aof_rewrite_in_progress") != "0"
            or persistence.get("aof_rewrite_scheduled") != "0"
            or persistence.get("aof_last_bgrewrite_status") != "ok"):
        raise ValueError("restore Redis persistence is not stable")
    redis = {"engine": engine, "server_version": release,
             "cli_version": version(tools["redis_cli"], env, r"(?:redis|valkey)-cli ([0-9]+\.[0-9]+\.[0-9]+)"),
             "database": 0, "appendonly": enabled,
             "tools": {name: records[tool] for name, tool in
                       (("server", "redis_server"), ("redis_cli", "redis_cli"), ("redis_check_rdb", "redis_check_rdb"))}}
    return {"postgres": pg, "redis": redis}, {"catalog": catalog, "tools": records}
