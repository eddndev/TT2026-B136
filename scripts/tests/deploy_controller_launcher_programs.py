"""Harmless controller and interpreter bodies for generation launcher tests."""

CHILD_API = r'''
import json
import os
from pathlib import Path
import runpy
import sys

launcher, root, entry, fingerprint, arguments = sys.argv[1:]
api = runpy.run_path(launcher, run_name="controller_launcher_fixture")
api["launch"](Path(root), entry, json.loads(arguments),
              expected_inventory_sha256=fingerprint)
'''

IDENTITY_API = CHILD_API.replace('api["launch"](', '''
identity = os.environ["CONTROLLER_FIXTURE_IDENTITY"]
if identity == "effective_uid":
    actual = os.geteuid()
    os.geteuid = lambda: actual + 1
elif identity == "effective_gid":
    actual = os.getegid()
    os.getegid = lambda: actual + 1
elif identity == "foreign_owner":
    actual = os.getuid()
    os.getuid = os.geteuid = lambda: actual + 1
api["launch"](''')

LATE_IMPORT = r'''
import hashlib
import json
import os
from pathlib import Path
import sys
import early

print(json.dumps({"ready": early.VALUE, "entry": ENTRY_GENERATION, "pid": os.getpid()}), flush=True)
if sys.stdin.readline(32) != "continue\n":
    raise RuntimeError("fixture continuation was not received")
import late
directory = Path(__file__).resolve().parent
inventory = {path.name: {"bytes": len(path.read_bytes()),
                        "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
             for path in sorted(directory.iterdir())}
print(json.dumps({"early": early.VALUE, "late": late.VALUE, "entry": ENTRY_GENERATION,
                  "file": __file__, "early_file": early.__file__, "late_file": late.__file__,
                  "inventory": inventory, "resolved_inode": directory.stat().st_ino,
                  "path": sys.path, "bytecode_disabled": sys.dont_write_bytecode}), flush=True)
'''

ARGUMENTS = r'''
import json
import os
import sys
import early

print(json.dumps({"argv": sys.argv, "file": __file__, "path": sys.path,
                  "cwd": os.getcwd(), "pid": os.getpid(), "early": early.VALUE,
                  "environment": os.environ.get("CONTROLLER_FIXTURE_VALUE"),
                  "bytecode_disabled": sys.dont_write_bytecode}), flush=True)
'''

REJECT_SENTINEL = r'''
import os
from pathlib import Path
Path(os.environ["CONTROLLER_FIXTURE_MARKER"]).write_text("unapproved source executed\n")
'''

LIFECYCLE_ENTRY = r'''
import sys
if sys.argv[1] == "error":
    raise RuntimeError("fixture entry failed")
if sys.argv[1] == "exit":
    raise SystemExit(23)
'''

LIFECYCLE_DRIVER = r'''
import json
import os
from pathlib import Path
import runpy
import sys

launcher, root, fingerprint, outcome = sys.argv[1:]
api = runpy.run_path(launcher, run_name="controller_launcher_fixture")

def descriptors():
    result = {}
    for name in os.listdir("/proc/self/fd"):
        try:
            row = os.fstat(int(name))
        except OSError:
            continue
        result[name] = [row.st_dev, row.st_ino, row.st_mode]
    return result

before, arguments, paths = descriptors(), sys.argv[:], sys.path[:]
kind, status = "returned", None
try:
    api["launch"](Path(root), "runtime.py", [outcome],
                  expected_inventory_sha256=fingerprint)
except SystemExit as error:
    kind, status = "SystemExit", error.code
except RuntimeError:
    kind = "RuntimeError"
print(json.dumps({"kind": kind, "status": status, "before": before, "after": descriptors(),
                  "argv_restored": sys.argv == arguments, "path_restored": sys.path == paths}))
'''

EXEC_ENTRY = r'''
import json
import os
from pathlib import Path
import sys

directory = Path(__file__).parent
if directory.parent != Path("/proc/self/fd"):
    raise RuntimeError("entry was not loaded through a pinned directory")
descriptor = int(directory.name)
row = os.fstat(descriptor)
if os.get_inheritable(descriptor):
    raise RuntimeError("generation descriptor must be close-on-exec")
probe = """
import json, os, sys
descriptor, device, inode = map(int, sys.argv[1:4])
try:
    row = os.fstat(descriptor)
    inherited = (row.st_dev, row.st_ino) == (device, inode)
except OSError:
    inherited = False
print(json.dumps({'inherited': inherited, 'pid': os.getpid(), 'arguments': sys.argv[4:],
                  'environment': os.environ.get('CONTROLLER_FIXTURE_VALUE')}))
"""
os.execve(sys.executable, [sys.executable, "-I", "-B", "-S", "-c", probe, str(descriptor),
                          str(row.st_dev), str(row.st_ino), *sys.argv[1:]], dict(os.environ))
'''

SIGNAL_ENTRY = r'''
import json
import os
import signal
print(json.dumps({"ready": True, "pid": os.getpid()}), flush=True)
signal.pause()
'''
