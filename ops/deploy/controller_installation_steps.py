"""Durable publication receipts and explicitly authorized controller reopening."""
from pathlib import Path

from backup_manifest_files import present
from backup_manifest_schema import keys
import controller_approval as approval
import controller_gate as gate
import controller_gate_files as gate_files
import controller_installation_files as inputs
import controller_publication as publication
import controller_publication_files as files
import controller_publication_record as publication_records
import controller_unit_commands as commands
import restore_quiesce as quiesce
from runtime import Runtime


UNITS = quiesce.UNITS


def selected(current):
    path = current.root / "maintenance/controllers/approved.json"
    observed, raw = files.read(path, 65536)
    value = approval.approvals.parse(raw)
    if (value["root"] != current.intent["root"] or value["operation_id"] != current.operation_id
            or value["installed_sha256"] != current.intent["candidate"]["installed_sha256"]
            or value["previous_approval_sha256"] != current.intent["previous_approval_sha256"]):
        raise ValueError("installation approval selection differs")
    return value, observed["sha256"]


def orientation(current, value):
    """Recognize only the exact candidate/mask exchange; retain original slots."""
    result = {}
    record = gate_files.read(current.operation / "journal.json")
    keys(record, ("format", "version", "operation_id", "target_sha256", "target", "root", "lock",
                  "unit_directory", "directories", "units", "state"))
    if (record["format"] != "qadra-controller-entry-gate" or type(record["version"]) is not int
            or record["version"] != 1 or record["operation_id"] != current.operation_id or record["target"] != current.target
            or record["target_sha256"] != current.target_digest or record["root"] != current.intent["root"]
            or record["lock"] != current.intent["lock"] or record["state"] != "gated"
            or record["unit_directory"] != current.intent["unit_directory"]):
        raise ValueError("installation retained gate belongs to another operation")
    active = {"operation_id": current.operation_id, "target_sha256": current.target_digest}
    marked = gate._active(current.operation.parent / "active.json", active)
    if not marked and current.record["state"] != "installed":
        raise ValueError("installation lost its active entry gate")
    keys(record["directories"], gate_files.directories(current.root, current.operation))
    keys(record["units"], UNITS)
    for name, path in gate_files.directories(current.root, current.operation).items():
        files.unchanged(path, record["directories"][name])
    gate.targets.attest_originals(record["units"], current.target)
    for name in ("slots", "candidates"):
        if set(path.name for path in (current.operation / name).iterdir()) != set(UNITS):
            raise ValueError("installation retained unit inventory changed")
    for unit in UNITS:
        fragment, candidate = current.units / unit, current.operation / "candidates" / unit
        if gate_files.regular(current.operation / "slots" / unit) != record["units"][unit]["original"]:
            raise ValueError("installation retained original changed")
        masked = fragment.is_symlink()
        source, mask = (candidate, fragment) if masked else (fragment, candidate)
        expected = value["candidates"][unit]
        keys(expected, ("identity", "bytes", "sha256"))
        files.unchanged(source, expected["identity"], directory=False)
        if (files.read(source, 65536)[0] != {key: expected[key] for key in ("bytes", "sha256")}
                or gate_files.mask(mask) != record["units"][unit]["mask"]):
            raise ValueError("installation candidate or its exact retained mask changed")
        result[unit] = "masked" if masked else "original"
    return result


def closed(current):
    value = gate_files.read(current.closed_path)
    keys(value, ("format", "version", "operation_id", "root", "intent_sha256", "target_sha256",
                 "stop_sha256", "approval_sha256", "installed_sha256", "launcher", "candidates"))
    if (value["format"] != "qadra-controller-installation-closed" or type(value["version"]) is not int
            or value["version"] != 1 or value["operation_id"] != current.operation_id
            or value["root"] != current.intent["root"] or value["intent_sha256"] != current.digest
            or value["target_sha256"] != current.target_digest
            or value["installed_sha256"] != current.intent["candidate"]["installed_sha256"]
            or value["stop_sha256"] != files.read(current.operation / "stop.json", 16384)[0]["sha256"]):
        raise ValueError("installation closed receipt differs from the exact intention")
    _, _, stopped = quiesce.saved(current.root, current.operation_id, current.target_digest, current.target,
                                  path=current.operation / "stop.json")
    if stopped["state"] != "stopped" or set(stopped["units"]) != set(UNITS):
        raise ValueError("installation closed receipt lacks exact captured exits")
    chosen, digest = selected(current)
    if value["approval_sha256"] != digest:
        raise ValueError("installation closed receipt approval changed")
    path = current.root / "maintenance/controllers" / current.operation_id
    published = approval._publication(current.root, path, current.intent["root"], current.intent["lock"],
                                     chosen["publication_sha256"], value["installed_sha256"])
    if publication_records.orientation(published, current.root, path, current.intent["lock"]) != "installed":
        raise ValueError("installation closed receipt lost its published sources")
    expected_launcher = value["launcher"]
    keys(expected_launcher, ("identity", "bytes", "sha256"))
    files.unchanged(current.root / "controller_launcher.py", expected_launcher["identity"], directory=False)
    if inputs.launcher(current.root, current.intent, install=True) != expected_launcher:
        raise ValueError("installation closed launcher changed")
    keys(value["candidates"], UNITS)
    orientation(current, value)
    inputs.cache_at(current.operation / "retained-cache", current.intent["cache"])
    inputs.cache_at(current.root / "tools/__pycache__", None)
    return value


def publish(current):
    if present(current.closed_path):
        value = closed(current)
        if set(orientation(current, value).values()) != {"masked"}:
            raise ValueError("installation closure contains reopened units")
        current.save("closed")
        return value
    current.live()
    inputs.retain_cache(current.root, current.operation, current.intent)
    publication._publish_locked(current.root, current.operation_id, Path(current.intent["candidate"]["path"]),
        expected_manifest_sha256=current.intent["candidate"]["manifest_sha256"],
        expected_previous_sha256=current.intent["previous_sources"]["sha256"])
    current.remaining()
    operation = current.root / "maintenance/controllers" / current.operation_id
    approval._approve_locked(current.root, current.operation_id,
        expected_publication_sha256=files.read(operation / "journal.json", 256 * 1024)[0]["sha256"],
        expected_installed_sha256=current.intent["candidate"]["installed_sha256"],
        expected_previous_approval_sha256=current.intent["previous_approval_sha256"])
    current.save("published")
    chosen, digest = selected(current)
    rendered = commands.render_units(current.root, current.originals, chosen,
        expected_approval_sha256=digest,
        expected_root_identity={name: current.intent["root"][name] for name in ("uid", "device", "inode")},
        python_executable=Path(current.intent["python"]["path"]),
        launcher_path=current.root / "controller_launcher.py",
        legacy_absent_prestarts=current.intent["legacy_absent_prestarts"])
    launcher = inputs.launcher(current.root, current.intent, install=True)
    directory = current.operation / "candidates"
    files.ensure_directory(directory)
    if not set(path.name for path in directory.iterdir()).issubset(set(UNITS)):
        raise ValueError("installation refuses unrelated candidate files")
    candidates = {}
    for unit, raw in rendered.items():
        path = directory / unit
        if not present(path):
            files.write_new(path, raw)
        if files.read(path, 65536)[1] != raw:
            raise ValueError("installation candidate bytes changed")
        gate_files.sync_file(path)
        candidates[unit] = {"identity": files.identity(path, directory=False), **files.read(path, 65536)[0]}
    files.sync_directory(directory)
    current.absence(current.orientation(), complete=True)
    value = {"format": "qadra-controller-installation-closed", "version": 1,
        "operation_id": current.operation_id, "root": current.intent["root"],
        "intent_sha256": current.digest, "target_sha256": current.target_digest,
        "stop_sha256": files.read(current.operation / "stop.json", 16384)[0]["sha256"],
        "approval_sha256": digest, "installed_sha256": current.intent["candidate"]["installed_sha256"],
        "launcher": launcher, "candidates": candidates}
    current.remaining()
    gate_files.save(current.closed_path, value, files.sync_directory)
    current.save("closed")
    return closed(current)


def authority(current, value, path, digest, *, save):
    authority_value = inputs.pinned(path, digest, 4096)
    expected = {"format": "qadra-controller-reopen", "version": 1,
        "operation_id": current.operation_id, "root": current.intent["root"],
        "intent_sha256": current.digest, "closed_sha256": files.read(current.closed_path, 65536)[0]["sha256"]}
    if authority_value != expected or type(authority_value["version"]) is not int:
        raise ValueError("installation reopening authority differs from the closed receipt")
    observed, raw = files.read(path, 4096)
    if observed["sha256"] != digest:
        raise ValueError("installation reopening authority changed")
    destination = current.operation / "reopen.json"
    if present(destination):
        if files.read(destination, 4096)[1] != raw:
            raise ValueError("installation reopening already records another authority")
    elif save:
        files.write_new(destination, raw)
    if save:
        gate_files.sync_file(destination)
        files.sync_directory(current.operation)


def reopen(current, value, path, digest):
    authority(current, value, path, digest, save=True)
    if current.record["state"] != "installed":
        current.save("reopening")
    for unit in UNITS:
        current.live()
        if orientation(current, value)[unit] == "masked":
            gate_files.exchange(current.operation / "candidates" / unit, current.units / unit)
        files.sync_directory(current.units)
        files.sync_directory(current.operation / "candidates")
        orientation(current, value)
    current.command("daemon-reload")
    orientations = orientation(current, value)
    if set(orientations.values()) != {"original"}:
        raise ValueError("installation reopening still contains masks")
    for group in (UNITS[2:], UNITS[1:2], UNITS[:1]):
        current.live()
        rows = current.rows(orientations)
        active = []
        for unit in group:
            row = rows[unit]
            quiesce.owned_processes(current.target, current.deadline, unit, row)
            if row["ActiveState"] == "inactive":
                active.append(unit)
        if active:
            current.command("start", active)
        rows = current.rows(orientations)
        for unit in group:
            if rows[unit]["ActiveState"] != "active":
                raise ValueError("installation service did not become active")
            quiesce.owned_processes(current.target, current.deadline, unit, rows[unit])
    current.live()
    Runtime(current.root).check(inputs.release(current.root, current.intent), deadline=current.deadline)
    current.remaining()
    closed(current)
    current.save("installed")
    marker = current.operation.parent / "active.json"
    expected = {"operation_id": current.operation_id, "target_sha256": current.target_digest}
    if present(marker):
        if gate_files.read(marker) != expected:
            raise ValueError("installation refuses another active gate")
        owned = files.identity(marker, directory=False)
        files.unchanged(marker, owned, directory=False)
        marker.unlink()
    files.sync_directory(marker.parent)
