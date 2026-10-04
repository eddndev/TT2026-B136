"""Retain local manager units and read typed exit evidence on one connection."""
import ctypes as C
import math
import os
from pathlib import Path
import re
import stat
from time import monotonic

from restore_quiesce_target import UNITS


POINTER = C.c_void_p
MANAGER = b"/org/freedesktop/systemd1"
INTERFACE = b"org.freedesktop.systemd1.Manager"
DBUS = b"org.freedesktop.DBus"
ERROR = "local service reference observation failed"
FIELDS = {"pid": ("ExecMainPID", b"u", C.c_uint32),
          "code": ("ExecMainCode", b"i", C.c_int32),
          "status": ("ExecMainStatus", b"i", C.c_int32),
          "started_usec": ("ExecMainStartTimestampMonotonic", b"t", C.c_uint64),
          "exited_usec": ("ExecMainExitTimestampMonotonic", b"t", C.c_uint64)}


def require(value):
    if not value:
        raise RuntimeError(ERROR)


def identity(info):
    return (info.st_dev, info.st_ino, info.st_uid, info.st_mode, info.st_size, info.st_mtime_ns)


def trusted(path):
    pending, current, observed = list(path.parts[1:]), Path("/"), []
    for _ in range(128):
        info = current.lstat()
        require(stat.S_ISDIR(info.st_mode) and info.st_uid == 0 and not info.st_mode & 0o022)
        observed.append((current, identity(info)))
        require(bool(pending))
        part = pending.pop(0)
        if part in (".", ".."):
            current = current.parent if part == ".." else current
            continue
        candidate = current / part
        info = candidate.lstat()
        require(info.st_uid == 0)
        observed.append((candidate, identity(info)))
        if stat.S_ISLNK(info.st_mode):
            link = Path(os.readlink(candidate))
            require(len(str(link)) <= 4096)
            link = link if link.is_absolute() else current / link
            pending, current = list(link.parts[1:]) + pending, Path("/")
        elif pending:
            require(stat.S_ISDIR(info.st_mode) and not info.st_mode & 0o022)
            current = candidate
        else:
            require(stat.S_ISREG(info.st_mode) and not info.st_mode & 0o022
                    and 0 < info.st_size <= 16 * 1024 * 1024)
            return candidate, info, observed
    raise RuntimeError(ERROR)


def library():
    require(not any(os.environ.get(name) for name in ("LD_LIBRARY_PATH", "LD_PRELOAD", "LD_AUDIT")))
    machine = os.uname().machine
    multiarch = {"x86_64": "x86_64-linux-gnu", "aarch64": "aarch64-linux-gnu",
                 "armv7l": "arm-linux-gnueabihf", "ppc64le": "powerpc64le-linux-gnu",
                 "riscv64": "riscv64-linux-gnu"}.get(machine)
    locations = [Path(prefix) / multiarch for prefix in ("/usr/lib", "/lib")] if multiarch else []
    locations += [Path(prefix) for prefix in ("/usr/lib64", "/lib64", "/usr/lib", "/lib")]
    selected = next((path / "libsystemd.so.0" for path in locations
                     if os.path.lexists(path / "libsystemd.so.0")), None)
    require(selected is not None)
    path, before, observed = trusted(selected)
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK)
    try:
        require(identity(os.fstat(descriptor)) == identity(before))
        size, header = 0, b""
        while True:
            block = os.read(descriptor, 65536)
            if not block:
                break
            header = header or block[:4]
            size += len(block)
            require(size <= before.st_size)
        require(header == b"\x7fELF" and size == before.st_size
                and identity(os.fstat(descriptor)) == identity(before))
        loaded = C.CDLL(f"/proc/self/fd/{descriptor}", use_errno=True)
        require(all(identity(item.lstat()) == value for item, value in observed)
                and identity(os.fstat(descriptor)) == identity(before))
    finally:
        os.close(descriptor)
    signatures = {
        "sd_bus_new": (C.c_int, [C.POINTER(POINTER)]),
        "sd_bus_set_address": (C.c_int, [POINTER, C.c_char_p]),
        "sd_bus_set_bus_client": (C.c_int, [POINTER, C.c_int]),
        "sd_bus_set_allow_interactive_authorization": (C.c_int, [POINTER, C.c_int]),
        "sd_bus_set_exit_on_disconnect": (C.c_int, [POINTER, C.c_int]),
        "sd_bus_set_method_call_timeout": (C.c_int, [POINTER, C.c_uint64]),
        "sd_bus_start": (C.c_int, [POINTER]),
        "sd_bus_is_open": (C.c_int, [POINTER]),
        "sd_bus_get_fd": (C.c_int, [POINTER]),
        "sd_bus_close_unref": (POINTER, [POINTER]),
        "sd_bus_message_new_method_call": (C.c_int, [POINTER, C.POINTER(POINTER),
                                                    C.c_char_p, C.c_char_p, C.c_char_p, C.c_char_p]),
        "sd_bus_message_append_basic": (C.c_int, [POINTER, C.c_char, POINTER]),
        "sd_bus_call": (C.c_int, [POINTER, POINTER, C.c_uint64, POINTER, C.POINTER(POINTER)]),
        "sd_bus_message_get_signature": (C.c_char_p, [POINTER, C.c_int]),
        "sd_bus_message_get_sender": (C.c_char_p, [POINTER]),
        "sd_bus_message_read_basic": (C.c_int, [POINTER, C.c_char, POINTER]),
        "sd_bus_message_enter_container": (C.c_int, [POINTER, C.c_char, C.c_char_p]),
        "sd_bus_message_exit_container": (C.c_int, [POINTER]),
        "sd_bus_message_at_end": (C.c_int, [POINTER, C.c_int]),
        "sd_bus_message_unref": (POINTER, [POINTER]),
    }
    for name, (result, arguments) in signatures.items():
        try:
            function = getattr(loaded, name)
        except AttributeError:
            raise RuntimeError(ERROR) from None
        function.restype, function.argtypes = result, arguments
    return loaded


class References:
    def __init__(self, environment, units, timeout):
        require(type(timeout) in (int, float) and math.isfinite(timeout) and 0 < timeout <= 300)
        self.deadline = monotonic() + timeout
        require(tuple(units) == UNITS and os.getuid() > 0 and os.getuid() == os.geteuid())
        runtime = f"/run/user/{os.getuid()}"
        expected = "unix:path=" + runtime + "/bus"
        require(environment["DBUS_SESSION_BUS_ADDRESS"] == expected and environment["XDG_RUNTIME_DIR"] == runtime)
        info = Path(runtime).lstat()
        require(stat.S_ISDIR(info.st_mode) and info.st_uid == os.getuid()
                and stat.S_IMODE(info.st_mode) == 0o700 and Path(runtime).resolve(strict=True) == Path(runtime))
        info = Path(runtime, "bus").lstat()
        require(stat.S_ISSOCK(info.st_mode) and info.st_uid == os.getuid())
        self.address, self.api = expected.encode("ascii"), library()
        self.bus, self.owner, self.held = POINTER(), None, []

    @property
    def manager(self):
        return self.owner.decode("ascii")

    def checked(self, value):
        require(value >= 0)
        return value

    def end(self, timeout):
        require(type(timeout) in (int, float) and math.isfinite(timeout) and 0 < timeout <= 300)
        return min(self.deadline, monotonic() + timeout)

    def call(self, destination, path, interface, member, arguments, signature=b"", reader=None, *, end):
        remaining = min(5.0, end - monotonic())
        require(remaining > 0)
        message, reply = POINTER(), POINTER()
        try:
            self.checked(self.api.sd_bus_message_new_method_call(self.bus, C.byref(message),
                         destination, path, interface, member))
            for argument in arguments:
                self.checked(self.api.sd_bus_message_append_basic(message, b"s", C.c_char_p(argument)))
            self.checked(self.api.sd_bus_call(self.bus, message, max(1, int(remaining * 1000000)),
                         None, C.byref(reply)))
            require(monotonic() < end and self.api.sd_bus_message_get_signature(reply, 1) == signature)
            if destination.startswith(b":"):
                require(self.api.sd_bus_message_get_sender(reply) == destination)
            value = reader(reply) if reader else None
            require(self.api.sd_bus_message_at_end(reply, 1) > 0)
            return value
        finally:
            if reply.value:
                self.api.sd_bus_message_unref(reply)
            if message.value:
                self.api.sd_bus_message_unref(message)

    def manager_owner(self, end):
        def read(reply):
            text = C.c_char_p()
            require(self.checked(self.api.sd_bus_message_read_basic(reply, b"s", C.byref(text))) > 0)
            require(text.value is not None and len(text.value) <= 256
                    and re.fullmatch(rb":[0-9]+\.[0-9]+", text.value))
            return text.value
        return self.call(DBUS, b"/org/freedesktop/DBus", DBUS, b"GetNameOwner",
                         (b"org.freedesktop.systemd1",), b"s", read, end=end)

    def check(self, *, timeout):
        require(self.api.sd_bus_is_open(self.bus) > 0 and self.held == list(UNITS))
        require(self.manager_owner(self.end(timeout)) == self.owner)

    def status(self, unit, *, timeout):
        require(unit in self.held)
        end = self.end(timeout)
        self.check(timeout=end - monotonic())
        encoded = "".join(character if character.isalnum() else f"_{ord(character):02x}" for character in unit)
        path = MANAGER + b"/unit/" + encoded.encode("ascii")

        def read_field(field, code, kind):
            def read(reply):
                require(self.checked(self.api.sd_bus_message_enter_container(reply, b"v", code)) > 0)
                value = kind()
                require(self.checked(self.api.sd_bus_message_read_basic(reply, code, C.byref(value))) > 0)
                require(self.api.sd_bus_message_at_end(reply, 0) > 0)
                self.checked(self.api.sd_bus_message_exit_container(reply))
                return value.value
            return self.call(self.owner, path, b"org.freedesktop.DBus.Properties", b"Get",
                             (b"org.freedesktop.systemd1.Service", field.encode("ascii")), b"v", read, end=end)

        first = {key: read_field(*field) for key, field in FIELDS.items()}
        require(first == {key: read_field(*field) for key, field in FIELDS.items()})
        self.check(timeout=end - monotonic())
        return first

    def __enter__(self):
        try:
            self.checked(self.api.sd_bus_new(C.byref(self.bus)))
            self.checked(self.api.sd_bus_set_address(self.bus, self.address))
            self.checked(self.api.sd_bus_set_bus_client(self.bus, 1))
            self.checked(self.api.sd_bus_set_allow_interactive_authorization(self.bus, 0))
            self.checked(self.api.sd_bus_set_exit_on_disconnect(self.bus, 0))
            self.checked(self.api.sd_bus_set_method_call_timeout(self.bus, 5000000))
            self.checked(self.api.sd_bus_start(self.bus))
            self.owner = self.manager_owner(self.deadline)
            descriptor = self.checked(self.api.sd_bus_get_fd(self.bus))
            require(not os.get_inheritable(descriptor))
            for unit in UNITS:
                self.call(self.owner, MANAGER, INTERFACE, b"RefUnit", (unit.encode("ascii"),), end=self.deadline)
                self.held.append(unit)
            self.check(timeout=self.deadline - monotonic())
            return self
        except BaseException:
            self.close()
            raise

    def close(self):
        failed = False
        try:
            end = monotonic() + 8
            for unit in reversed(self.held):
                try:
                    self.call(self.owner, MANAGER, INTERFACE, b"UnrefUnit", (unit.encode("ascii"),),
                              end=min(end, monotonic() + 2))
                except (OSError, RuntimeError, ValueError):
                    failed = True
        finally:
            self.held.clear()
            if self.bus.value:
                self.api.sd_bus_close_unref(self.bus)
                self.bus = POINTER()
        require(not failed)

    def __exit__(self, _kind, _value, _traceback):
        self.close()


def retain_units(environment, units, *, timeout):
    return References(environment, units, timeout)
