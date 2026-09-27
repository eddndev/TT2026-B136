"""Exercise the real coverage tool and slot runner on a small Rust workspace."""

import argparse
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
        let slot = env::var("NEXTEST_TEST_GLOBAL_SLOT").unwrap();
        assert!(redis.ends_with(&format!("/{slot}")));
        let result = Command::new("redis-cli").args(["-u", &redis, "ping"])
            .output().unwrap();
        assert!(result.status.success());
        assert_eq!(String::from_utf8(result.stdout).unwrap().trim(), "PONG");
        let directory = env::var("TT_SMOKE_BARRIER").unwrap();
        let ready = std::path::Path::new(&directory).join(&slot);
        std::fs::OpenOptions::new().create_new(true).write(true).open(ready).unwrap();
        let slots: usize = env::var("TT_SMOKE_SLOTS").unwrap().parse().unwrap();
        let start = std::time::Instant::now();
        while std::fs::read_dir(&directory).unwrap().count() != slots {
            assert!(start.elapsed().as_secs() < 30, "backend slots did not run concurrently");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let database = env::var("CASE_TEST_DATABASE_URL").unwrap();
        let status = Command::new("psql").args([&database, "-Xq", "-v", "ON_ERROR_STOP=1",
            "-c", "SET lock_timeout='250ms'; SELECT pg_advisory_lock(280603412820); SELECT pg_sleep(0.5)"])
            .status().unwrap();
        assert!(status.success(), "independent slots must not contend for the audit key");
    }
    // BACKEND_TESTS
}
'''


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--slots", type=int, choices=range(1, 17), default=1)
    args = parser.parse_args()
    for key in ("IDENTITY_TEST_DATABASE_URL", "IDENTITY_TEST_REDIS_URL"):
        if not os.environ.get(key):
            raise RuntimeError(f"isolated services required: {key}")
    with tempfile.TemporaryDirectory(prefix="nextest-smoke-") as temp:
        root = Path(temp)
        (root / "crates/domain/src").mkdir(parents=True)
        (root / "Cargo.toml").write_text('[workspace]\nmembers=["crates/domain"]\nresolver="2"\n')
        (root / "crates/domain/Cargo.toml").write_text(
            '[package]\nname="domain"\nversion="0.1.0"\nedition="2021"\n')
        tests = "\n".join(f"#[test] fn backend_{slot}() {{ isolated_backends_are_available(); }}"
                          for slot in range(args.slots))
        (root / "crates/domain/src/lib.rs").write_text(SOURCE.replace("// BACKEND_TESTS", tests))
        (root / "barrier").mkdir()
        for relative in ("scripts/ci_nextest.py", "scripts/ci_test_slot.py",
                         "scripts/ci_test_shard.py", "scripts/ci-coverage.py",
                         ".config/nextest.toml"):
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, path)
        subprocess.run(["cargo", "generate-lockfile", "--offline"], cwd=root, check=True)
        environment = {**os.environ, "CARGO_TARGET_DIR": str(root / "target"),
                       "CARGO_BUILD_JOBS": "1", "RUST_TEST_THREADS": "1",
                       "TT_SMOKE_BARRIER": str(root / "barrier"),
                       "TT_SMOKE_SLOTS": str(args.slots)}
        subprocess.run(["python3", "-B", "scripts/ci_nextest.py", "--slots", str(args.slots)],
                       cwd=root, env=environment, check=True)
        report = (root / "coverage-shard-1.info").read_text()
        assert "DA:2," in report and "DA:3," in report, report
        cases = ET.parse(root / "output/ci-nextest-junit.xml").findall(".//testcase")
        assert len(cases) == args.slots + 2
        assert all(case.find("failure") is None and case.find("error") is None for case in cases)
        print(f"PASS: {len(cases)} real tests, {args.slots} isolated slots, fresh coverage and JUnit")


if __name__ == "__main__":
    main()
