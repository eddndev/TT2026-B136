"""Render private unit candidates from externally approved controller evidence."""
from pathlib import Path
import re

from backup_manifest_schema import hexadecimal
import controller_approval_record as records


UNITS = ("qadra-api.service", "qadra-web.service", "qadra-postgres.service", "qadra-redis.service")
MAX_UNIT_BYTES = 64 * 1024


def _path(value):
    if (not isinstance(value, Path) or value.anchor != "/" or str(value) == "/"
            or not re.fullmatch(r"/[A-Za-z0-9_./-]+", str(value))
            or ".." in value.parts):
        raise ValueError("controller unit paths must be absolute without metacharacters")
    return str(value)


def _commands(raw):
    if type(raw) is not bytes or not 0 < len(raw) <= MAX_UNIT_BYTES or not raw.isascii() or b"\0" in raw:
        raise ValueError("controller unit is not bounded ASCII bytes")
    section, services, commands = None, 0, {}
    lines = raw.splitlines(keepends=True)
    for index, line in enumerate(lines):
        stripped = line.strip()
        if not stripped or stripped.startswith((b"#", b";")):
            continue
        if stripped.endswith(b"\\"):
            raise ValueError("controller unit contains an ambiguous continuation")
        if stripped.startswith(b"["):
            if not stripped.endswith(b"]"):
                raise ValueError("controller unit section is invalid")
            section = stripped
            services += section == b"[Service]"
            if services > 1:
                raise ValueError("controller unit has multiple service sections")
            continue
        key, separator, value = stripped.partition(b"=")
        key = key.strip()
        if not separator:
            raise ValueError("controller unit contains an unsupported directive")
        if key.startswith(b"Exec"):
            if (section != b"[Service]" or not value.strip()
                    or key not in (b"ExecStart", b"ExecStartPre") or key in commands):
                raise ValueError("controller unit has unknown or repeated execution directives")
            commands[key] = index
    if services != 1 or b"ExecStart" not in commands:
        raise ValueError("controller unit lacks one explicit service command")
    return lines, commands


def _unit(name, raw, root, python, prefix):
    lines, commands = _commands(raw)
    if name == "qadra-api.service":
        if set(commands) != {b"ExecStart"}:
            raise ValueError("API unit contains an unreviewed prestart command")
        directive, entry = b"ExecStart", "runtime.py"
    else:
        if set(commands) != {b"ExecStart", b"ExecStartPre"}:
            raise ValueError("controller unit lacks its exact prestart command")
        directive = b"ExecStartPre"
        entry = "runtime.py" if name == "qadra-web.service" else "restore_fence.py"
    suffix = " --wait-api" if name == "qadra-web.service" else ""
    position = commands[directive]
    original = lines[position]
    content = original.rstrip(b"\r\n")
    ending = original[len(content):]
    expected = f"{directive.decode()}={python} {root}/tools/{entry} {root}{suffix}".encode("ascii")
    if content != expected:
        raise ValueError("controller unit command differs from its captured form")
    replacement = f"{directive.decode()}={prefix}{entry} -- {root}{suffix}".encode("ascii")
    lines[position] = replacement + ending
    return b"".join(lines)


def _original_units(root, current_units, python_executable, legacy_absent_prestarts=()):
    """Validate captured commands and explicitly approved legacy prestart absence."""
    root_name, python = _path(root), _path(python_executable)
    if type(current_units) is not dict or set(current_units) != set(UNITS):
        raise ValueError("controller unit inventory is incomplete or unknown")
    allowed = ("qadra-postgres.service", "qadra-redis.service")
    if (type(legacy_absent_prestarts) not in (list, tuple)
            or len(set(legacy_absent_prestarts)) != len(legacy_absent_prestarts)
            or any(name not in allowed for name in legacy_absent_prestarts)):
        raise ValueError("legacy prestart absence requires exact database unit names")
    result = dict(current_units)
    for name, raw in current_units.items():
        lines, commands = _commands(raw)
        if name in legacy_absent_prestarts:
            if set(commands) != {b"ExecStart"}:
                raise ValueError("legacy prestart absence differs from its approval")
            positions = [i for i, line in enumerate(lines) if line.startswith(b"MemoryMax=")]
            if len(positions) != 1:
                raise ValueError("legacy unit lacks its unambiguous resource limit")
            lines.insert(positions[0], f"ExecStartPre={python} {root_name}/tools/restore_fence.py {root_name}\n".encode())
            result[name] = b"".join(lines)
        _unit(name, result[name], root_name, python, "")
    return result


def render_units(root, current_units, approval, *, expected_approval_sha256,
                 expected_root_identity, python_executable, launcher_path,
                 legacy_absent_prestarts=()):
    """Return candidates only; the caller validates current files and manager state."""
    root_name, python, launcher = _path(root), _path(python_executable), _path(launcher_path)
    if launcher_path.is_relative_to(root / "tools"):
        raise ValueError("controller launcher must remain outside published sources")
    hexadecimal(expected_approval_sha256, 64)
    records.identity(expected_root_identity)
    raw = records.validate(approval)
    if records.digest(raw) != expected_approval_sha256:
        raise ValueError("controller approval differs from its external expected digest")
    if approval["root"] != {"path": root_name, **expected_root_identity}:
        raise ValueError("controller approval differs from the external target identity")
    current_units = _original_units(root, current_units, python_executable, legacy_absent_prestarts)
    prefix = (f"{python} -I -B -S {launcher} --root {root_name} "
              f"--inventory-sha256 {approval['installed_sha256']} --entrypoint ")
    return {name: _unit(name, current_units[name], root_name, python, prefix) for name in UNITS}
