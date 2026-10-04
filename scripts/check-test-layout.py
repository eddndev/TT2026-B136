"""Every integration source must belong to exactly one executable test suite."""
from collections import Counter
from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def count_sources(path, sources, counts, ancestors=()):
    if path in ancestors:
        raise RuntimeError(f"{path.relative_to(ROOT)}: cyclic explicit module inclusion")
    if path in sources:
        counts[path] += 1
    for relative in re.findall(r'#\[path = "([^"]+)"\]', path.read_text()):
        child = (path.parent / relative).resolve()
        count_sources(child, sources, counts, ancestors + (path,))


def check(crate):
    manifest = tomllib.loads((crate / "Cargo.toml").read_text())
    if manifest["package"].get("autotests", True):
        return len(list((crate / "tests").glob("*.rs")))
    sources = {path.resolve() for path in (crate / "tests").glob("*.rs")}
    counts = Counter()
    for target in manifest.get("test", []):
        path = (crate / target["path"]).resolve()
        count_sources(path, sources, counts)
    errors = [f"{path.relative_to(ROOT)}: registered {counts[path]} times"
              for path in sorted(sources) if counts[path] != 1]
    if errors:
        raise RuntimeError("\n".join(errors))
    return len(manifest["test"])


if __name__ == "__main__":
    total = 0
    for crate in sorted((ROOT / "crates").iterdir()):
        if (crate / "Cargo.toml").is_file():
            count = check(crate)
            total += count
            print(f"{crate.name}: {count} integration executables")
    print(f"Total: {total} integration executables; all source files registered once")
