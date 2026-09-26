"""Exercise the real coverage tool and slot runner on a small Rust workspace."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
SOURCE = '''
pub fn classify(value: bool) -> u32 {
    if value { 1 } else { 2 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_branch() { assert_eq!(classify(true), 1); }
    #[test]
    fn second_branch() { assert_eq!(classify(false), 2); }
    #[test]
    fn isolated_backends_are_available() {
        use std::{env, process::Command};
        let mut urls = std::collections::HashSet::new();
        for key in ["IDENTITY_TEST_DATABASE_URL", "CASE_TEST_DATABASE_URL",
                    "DOCUMENT_TEST_DATABASE_URL"] {
            let url = env::var(key).unwrap();
            let result = Command::new("psql").args([&url, "-XAt", "-c",
                "SELECT current_database()"] ).output().unwrap();
            assert!(result.status.success());
            assert!(String::from_utf8(result.stdout).unwrap().starts_with("ci_"));
            assert!(urls.insert(url));
        }
        let redis = env::var("IDENTITY_TEST_REDIS_URL").unwrap();
        assert!(redis.ends_with("/0"));
        let result = Command::new("redis-cli").args(["-u", &redis, "ping"])
            .output().unwrap();
        assert!(result.status.success());
        assert_eq!(String::from_utf8(result.stdout).unwrap().trim(), "PONG");
    }
}
'''


def main():
    for key in ("IDENTITY_TEST_DATABASE_URL", "IDENTITY_TEST_REDIS_URL"):
        if not os.environ.get(key):
            raise RuntimeError(f"isolated services required: {key}")
    with tempfile.TemporaryDirectory(prefix="nextest-smoke-") as temp:
        root = Path(temp)
        (root / "crates/domain/src").mkdir(parents=True)
        (root / "Cargo.toml").write_text('[workspace]\nmembers=["crates/domain"]\nresolver="2"\n')
        (root / "crates/domain/Cargo.toml").write_text(
            '[package]\nname="domain"\nversion="0.1.0"\nedition="2021"\n')
        (root / "crates/domain/src/lib.rs").write_text(SOURCE)
        for relative in ("scripts/ci_nextest.py", "scripts/ci_test_slot.py",
                         "scripts/ci_test_shard.py", "scripts/ci-coverage.py",
                         ".config/nextest.toml"):
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, path)
        subprocess.run(["cargo", "generate-lockfile", "--offline"], cwd=root, check=True)
        environment = {**os.environ, "CARGO_TARGET_DIR": str(root / "target"),
                       "CARGO_BUILD_JOBS": "1", "RUST_TEST_THREADS": "1"}
        subprocess.run(["python3", "-B", "scripts/ci_nextest.py", "--slots", "1"],
                       cwd=root, env=environment, check=True)
        report = (root / "coverage-shard-1.info").read_text()
        assert "DA:2," in report and "DA:3," in report, report
        cases = ET.parse(root / "output/ci-nextest-junit.xml").findall(".//testcase")
        assert len(cases) == 3
        assert all(case.find("failure") is None and case.find("error") is None for case in cases)
        print("PASS: three real tests, isolated PostgreSQL/Redis, fresh coverage and JUnit")


if __name__ == "__main__":
    main()
