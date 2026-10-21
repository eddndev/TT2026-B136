"""Renew the CRL from a timer only near expiry; failures stay fenced for a person.

The policy and its limits are recorded in docs/adr/0080-scheduled-crl-renewal.md.
"""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import time
from urllib.request import Request, urlopen
import uuid

import crl_journal as journal
import crl_material as material


RECEIPTS = "maintenance/crl-schedule"
CONFIG = "config/crl-schedule.json"
UNITS = ("qadra-crl-renew.service", "qadra-crl-renew.timer", "qadra-crl-alert.service")
EXIT_BLOCKED = 2
EXIT_BUSY = 75
MAX_THRESHOLD_SECONDS = 6 * 86400
SAFE_PATH = re.compile(r"/[A-Za-z0-9_./-]+")
RESEND_ENDPOINT = "https://api.resend.com/emails"


def validate_policy(threshold_seconds, keep):
    if type(threshold_seconds) is not int or not 3600 <= threshold_seconds <= MAX_THRESHOLD_SECONDS:
        raise ValueError("renewal threshold must be between one hour and six days")
    if type(keep) is not int or not 1 <= keep <= 32:
        raise ValueError("retained scheduled backups must be between 1 and 32")


def record(root, receipt):
    for directory in (root / "maintenance", root / RECEIPTS):
        journal.directory(directory)
    name = f"{int(receipt['checked_at'])}-{uuid.uuid4().hex[:8]}.json"
    journal.save(root / RECEIPTS / name, receipt)


def _renew(root, runtime, receipt, threshold_seconds, keep):
    import renew_crl
    try:
        journal.guard_mutation(root)
    except (RuntimeError, ValueError):
        receipt["result"] = "blocked"
        return
    head = material.read_head(root)
    receipt.update(revision_before=head["revision"], crl_number_before=head["crl_number"],
                   valid_until_before=head["valid_until"])
    if head["valid_until"] - receipt["checked_at"] > threshold_seconds:
        receipt["result"] = "skipped"
        return
    if head["valid_until"] < head["crl_next_update"]:
        # The authority itself expires first; another list would not extend validity.
        receipt["result"] = "ca_expiring"
        return
    target, info = renew_crl.current_release(root)
    operation, prepared = renew_crl.prepared(root, target, info, head["revision"])
    receipt["operation_id"] = operation.name
    if runtime is None:
        from runtime import Runtime
        runtime = Runtime(root)
    result = renew_crl.continue_operation(root, operation, prepared, target, runtime)
    receipt.update(result="renewed", revision_after=result["revision"],
                   crl_number_after=result["crl_number"], backup=prepared["backup"]["path"])
    try:
        receipt["valid_until_after"] = material.read_head(root)["valid_until"]
        # The current backup has no receipt yet, so keep one fewer earlier backup.
        receipt["pruned"] = prune(root, keep - 1)
    except Exception as error:
        receipt["post_error"] = type(error).__name__


def run(root, threshold_seconds, keep, runtime=None):
    validate_policy(threshold_seconds, keep)
    root = Path(root).resolve(strict=True)
    receipt = {"checked_at": int(time.time()), "threshold_seconds": threshold_seconds}
    try:
        with journal.locked(root):
            _renew(root, runtime, receipt, threshold_seconds, keep)
    except BlockingIOError:
        receipt["result"] = "busy" if _far_from_expiry(root, receipt, threshold_seconds) else "busy_near_expiry"
    except KeyboardInterrupt:
        # The unit stops with SIGINT so renewal reinstalls its fence before exiting.
        receipt["result"] = "interrupted"
    except Exception as error:
        # Exception text from external commands may contain private details.
        receipt.update(result="failed", error=type(error).__name__)
    record(root, receipt)
    return receipt


def _far_from_expiry(root, receipt, threshold_seconds):
    try:
        head = material.read_head(root)
    except Exception:
        return False
    receipt["valid_until_before"] = head["valid_until"]
    return head["valid_until"] - receipt["checked_at"] > threshold_seconds


def exit_code(receipt):
    return {"renewed": 0, "skipped": 0, "busy": EXIT_BUSY, "blocked": EXIT_BLOCKED}.get(
        receipt.get("result"), 1)


def _scheduled_receipts(root):
    directory = root / RECEIPTS
    if not directory.is_dir() or directory.is_symlink():
        return []
    rows = []
    for path in directory.glob("*.json"):
        if path.is_symlink() or not path.is_file():
            continue
        try:
            value = journal.load(path)
        except (OSError, ValueError):
            continue
        checked = value.get("checked_at")
        if (value.get("result") == "renewed" and isinstance(value.get("operation_id"), str)
                and type(checked) is int):
            rows.append((checked, path.name, path, value))
    return sorted(rows, reverse=True)


def _removable(root, value):
    backup = value.get("backup")
    if not isinstance(backup, str) or value.get("backup_pruned"):
        return None
    path = Path(backup)
    backups = root / "backups"
    if (path.is_symlink() or backups.is_symlink() or not path.is_dir()
            or path.parent != backups or path.resolve(strict=True).parent != backups):
        return None
    try:
        operation = journal.operation(root, value["operation_id"])
        recorded = journal.load(operation / "journal.json")
    except (OSError, ValueError):
        return None
    if recorded.get("state") != "accepted" or (recorded.get("backup") or {}).get("path") != backup:
        return None
    return path


def prune(root, keep):
    """Remove only backups of accepted scheduled renewals beyond the newest kept ones."""
    removed = []
    for _, _, receipt_path, value in _scheduled_receipts(root)[max(keep, 0):]:
        path = _removable(root, value)
        if path is None:
            continue
        shutil.rmtree(path)
        journal.save(receipt_path, {**value, "backup_pruned": True})
        removed.append(value["operation_id"])
    return removed


def _safe_path(value, label):
    if not isinstance(value, str) or not SAFE_PATH.fullmatch(value) or ".." in value.split("/"):
        raise ValueError(f"{label} must be an absolute path without metacharacters")
    return value


def invocation(raw):
    """Read the launcher invocation that admitted this exact controller generation."""
    if type(raw) is not bytes or not raw.endswith(b"\0"):
        raise ValueError("launcher command line is unavailable")
    words = [part.decode("ascii") for part in raw[:-1].split(b"\0")]
    if len(words) < 12 or words[1:4] != ["-I", "-B", "-S"]:
        raise ValueError("controller was not started by the isolated launcher")
    launcher = words[4]
    options = words[5:words.index("--")] if "--" in words else []
    pairs = dict(zip(options[::2], options[1::2]))
    if (len(options) != 6 or set(pairs) != {"--root", "--inventory-sha256", "--entrypoint"}
            or pairs["--entrypoint"] != "renew_crl.py"):
        raise ValueError("controller invocation differs from the CRL entrypoint")
    if launcher != pairs["--root"].rstrip("/") + "/controller_launcher.py":
        raise ValueError("controller was not started by the approved launcher")
    return {"python": words[0], "launcher": launcher, "root": pairs["--root"],
            "inventory_sha256": pairs["--inventory-sha256"]}


def units(launched, threshold_hours, keep):
    validate_policy(threshold_hours * 3600 if type(threshold_hours) is int else None, keep)
    python = _safe_path(launched["python"], "interpreter")
    launcher = _safe_path(launched["launcher"], "launcher")
    root = _safe_path(launched["root"], "root")
    if launcher != root.rstrip("/") + "/controller_launcher.py":
        raise ValueError("launcher must be the approved one in the deployment root")
    pin = launched["inventory_sha256"]
    if not isinstance(pin, str) or not re.fullmatch(r"[0-9a-f]{64}", pin):
        raise ValueError("controller inventory digest must be canonical SHA-256")
    prefix = (f"{python} -I -B -S {launcher} --root {root} --inventory-sha256 {pin} "
              f"--entrypoint renew_crl.py -- --root {root}")
    hardening = "UMask=0077\nNoNewPrivileges=yes\nPrivateTmp=yes\n"
    return {
        "qadra-crl-renew.service": f"""[Unit]
Description=Qadra scheduled CRL renewal near expiry
OnFailure=qadra-crl-alert.service
After=qadra-postgres.service qadra-redis.service qadra-api.service

[Service]
Type=oneshot
WorkingDirectory={root}
ExecStart={prefix} auto --threshold-hours {threshold_hours} --keep-backups {keep}
SuccessExitStatus={EXIT_BUSY}
TimeoutStartSec=2400
KillSignal=SIGINT
KillMode=mixed
TimeoutStopSec=120
{hardening}""",
        "qadra-crl-renew.timer": """[Unit]
Description=Daily check of the Qadra CRL validity

[Timer]
OnCalendar=*-*-* 09:00:00 UTC
RandomizedDelaySec=1800
Persistent=true

[Install]
WantedBy=timers.target
""",
        "qadra-crl-alert.service": f"""[Unit]
Description=Qadra scheduled CRL renewal alert

[Service]
Type=oneshot
WorkingDirectory={root}
ExecStart={prefix} alert
TimeoutStartSec=60
{hardening}""",
    }


def install_units(root, threshold_hours, keep, *, raw=None, directory=None, command=subprocess.run):
    launched = invocation(raw if raw is not None else Path("/proc/self/cmdline").read_bytes())
    if Path(launched["root"]).resolve(strict=True) != Path(root).resolve(strict=True):
        raise ValueError("launcher root differs from the scheduled CRL root")
    rendered = units(launched, threshold_hours, keep)
    directory = Path(directory or Path.home() / ".config/systemd/user")
    directory.mkdir(parents=True, exist_ok=True)
    for name, content in rendered.items():
        journal.write(directory / name, content.encode("ascii"))
    command(["systemctl", "--user", "daemon-reload"], check=True)
    command(["systemctl", "--user", "enable", "--now", "qadra-crl-renew.timer"], check=True)
    return {"units": sorted(rendered), "inventory_sha256": launched["inventory_sha256"],
            "threshold_hours": threshold_hours, "keep_backups": keep}


def _dotenv(path):
    values = {}
    for line in journal.read(path, 64 * 1024).decode("utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = re.sub(r"^export\s+", "", key.strip())
        value = value.strip()
        if value[:1] in ("\"", "'"):
            end = value.find(value[0], 1)
            value = value[1:end] if end > 0 else value[1:]
        else:
            value = re.split(r"\s+#", value, maxsplit=1)[0].rstrip()
        values[key] = value
    return values


def _invocation_key():
    value = os.environ.get("MONITOR_INVOCATION_ID", "")
    return value if re.fullmatch(r"[0-9a-f]{32}|[0-9a-f-]{1,64}", value) and value else uuid.uuid4().hex


def alert(root):
    """Send one generic notice describing the latest scheduled outcome."""
    root = Path(root).resolve(strict=True)
    try:
        config = journal.load(root / CONFIG)
    except OSError as error:
        raise ValueError("scheduled CRL alert recipient is not configured") from error
    recipient = config.get("alert_to")
    if not isinstance(recipient, str) or not re.fullmatch(r"[^@\s<>]+@[^@\s<>]+", recipient):
        raise ValueError("scheduled CRL alert recipient is not configured")
    env = _dotenv(root / "data/.env")
    key = env.get("RESEND_API_KEY", "")
    sender = env.get("REPORT_EMAIL_FROM") or env.get("INVITATION_EMAIL_FROM")
    if not key or not sender:
        raise ValueError("scheduled CRL alert sender is not configured")
    rows = []
    for path in (root / RECEIPTS).glob("*.json") if (root / RECEIPTS).is_dir() else ():
        try:
            value = journal.load(path)
        except (OSError, ValueError):
            continue
        if type(value.get("checked_at")) is int:
            rows.append((value["checked_at"], path.stat().st_mtime_ns, value))
    summary = max(rows, key=lambda row: row[:2])[2] if rows else {"result": "unknown"}
    failure = summary.get("result") not in ("renewed", "skipped", "busy")
    lines = ["La renovaci\u00f3n programada de la CRL de Qadra requiere atenci\u00f3n.",
             f"Resultado registrado: {summary.get('result')}"]
    if not failure:
        lines.append("La ejecuci\u00f3n fallida no dej\u00f3 recibo; el anterior se muestra como referencia.")
    for name in ("MONITOR_SERVICE_RESULT", "MONITOR_EXIT_CODE", "MONITOR_EXIT_STATUS"):
        if re.fullmatch(r"[A-Za-z0-9_-]{1,32}", os.environ.get(name, "")):
            lines.append(f"{name}: {os.environ[name]}")
    for field in ("operation_id", "revision_before", "valid_until_before", "error"):
        if field in summary:
            lines.append(f"{field}: {summary[field]}")
    lines.append("Un administrador debe revisar el estado con renew_crl.py status antes de reanudar.")
    body = json.dumps({"from": sender, "to": [recipient],
                       "subject": "Qadra: renovaci\u00f3n de CRL pendiente de revisi\u00f3n",
                       "text": "\n".join(lines)}).encode("utf-8")
    request = Request(RESEND_ENDPOINT, data=body, method="POST", headers={
        "Authorization": f"Bearer {key}", "Content-Type": "application/json",
        # The provider's edge rejects the default urllib client signature.
        "User-Agent": "qadra-crl-alert/1",
        "Idempotency-Key": "qadra-crl-" + _invocation_key()})
    with urlopen(request, timeout=10) as response:
        if not 200 <= response.status < 300:
            raise RuntimeError("alert provider rejected the message")
        response.read(4096)
    return {"alerted": summary.get("result")}
