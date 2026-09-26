"""Union line hits from two isolated LLVM coverage campaigns."""

import argparse
import json
from pathlib import Path


def source_path(raw, root):
    marker = "/crates/"
    position = raw.find(marker)
    if position < 0:
        return None
    path = root / raw[position + 1:]
    if not path.is_file():
        raise ValueError(f"coverage source is missing: {path}")
    return path


def merge(paths, root):
    if len(paths) != 2 or paths[0] == paths[1]:
        raise ValueError("exactly two distinct coverage shards are required")
    merged = {}
    for path in paths:
        if not path.is_file() or not path.stat().st_size:
            raise ValueError(f"coverage shard is missing or empty: {path}")
        current = None
        recorded = 0
        for line in path.read_text().splitlines():
            if line.startswith("SF:"):
                current = source_path(line[3:], root)
            elif line.startswith("DA:"):
                if current is None:
                    continue
                fields = line[3:].split(",")
                number, hits = int(fields[0]), int(fields[1])
                if number <= 0 or hits < 0:
                    raise ValueError(f"invalid line coverage in {path}: {line}")
                lines = merged.setdefault(current, {})
                lines[number] = lines.get(number, False) or hits > 0
                recorded += 1
            elif line == "end_of_record":
                current = None
        if not recorded:
            raise ValueError(f"coverage shard has no workspace lines: {path}")
    files = []
    for path, lines in sorted(merged.items()):
        files.append({
            "filename": str(path),
            "summary": {"lines": {
                "count": len(lines),
                "covered": sum(lines.values()),
            }},
        })
    return {"data": [{"files": files}]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("first", type=Path)
    parser.add_argument("second", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    result = merge([args.first, args.second], Path.cwd())
    args.output.write_text(json.dumps(result, separators=(",", ":")) + "\n")
    print(f"Merged coverage for {len(result['data'][0]['files'])} workspace files")


if __name__ == "__main__":
    main()
