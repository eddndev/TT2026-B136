#!/usr/bin/env python3
"""Private fixed-command bridge called by the separate authenticated CA service."""
import argparse
import json
from pathlib import Path
import sys

# -I excludes the working directory; import only this installed script tree.
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "deploy"))
sys.path.insert(0, str(HERE))

import issuance
from native_executor import NativeExecutor
import portal


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("root", "inspector", "cli", "scripts"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--expected-root", required=True)
    args = parser.parse_args()
    try:
        raw = sys.stdin.buffer.read(65537)
        if not raw or len(raw) > 65536:
            raise ValueError("CA input bound")
        request = json.loads(raw, object_pairs_hook=issuance.unique_object)
        if not isinstance(request, dict) or set(request) != {"operator_id", "command"}:
            raise ValueError("CA command envelope")
        executor = NativeExecutor(args.inspector, args.cli, args.scripts)
        value = portal.execute(args.root, request["operator_id"], request["command"], executor,
                               args.expected_root)
        encoded = json.dumps({"ok": True, "value": value}, ensure_ascii=True)
        if len(encoded) > 3 * 1024 * 1024:
            raise RuntimeError("CA result bound")
        print(encoded)
    except BlockingIOError:
        print(json.dumps({"ok": False, "code": "busy"}))
    except (ValueError, LookupError, UnicodeError):
        print(json.dumps({"ok": False, "code": "invalid"}))
    except Exception:
        # No command output, native exception, path, password or key in HTTP.
        print(json.dumps({"ok": False, "code": "unavailable"}))


if __name__ == "__main__":
    main()
