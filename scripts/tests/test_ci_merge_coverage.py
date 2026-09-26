import importlib.util
import json
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "merge_coverage_shards", ROOT / "scripts" / "merge_coverage_shards.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class TestCiMergeCoverage(unittest.TestCase):
    def test_cli_accepts_both_legacy_and_dedicated_report_counts(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "crates/domain/src/lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text("one line\n")
            reports = [root / "one.info", root / "two.info"]
            for report in reports:
                report.write_text("SF:/runner/crates/domain/src/lib.rs\nDA:1,1\nend_of_record\n")
            for count in (1, 2):
                output = root / f"result-{count}.json"
                arguments = ["--expected-shards", "1"] if count == 1 else []
                subprocess.run([sys.executable, str(ROOT / "scripts/merge_coverage_shards.py"),
                                *arguments, *map(str, reports[:count]), str(output)],
                               cwd=root, stdout=subprocess.PIPE, check=True)
                self.assertEqual(len(json.loads(output.read_text())["data"][0]["files"]), 1)

    def test_merges_line_hits_from_distinct_hosts(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "crates" / "domain" / "src" / "lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text("line 1\nline 2\nline 3\n")
            first = root / "first.info"
            second = root / "second.info"
            first.write_text(
                "SF:/vps1/work/crates/domain/src/lib.rs\n"
                "DA:1,1\nDA:2,0\nDA:3,0\nend_of_record\n"
            )
            second.write_text(
                "SF:/vps2/work/crates/domain/src/lib.rs\n"
                "DA:1,0\nDA:2,2\nDA:3,0\nend_of_record\n"
            )
            merged = MODULE.merge([first, second], root)
            self.assertEqual(len(merged["data"][0]["files"]), 1)
            entry = merged["data"][0]["files"][0]
            self.assertEqual(entry["filename"], str(source))
            self.assertEqual(entry["summary"]["lines"], {"count": 3, "covered": 2})

    def test_missing_shard_fails_instead_of_reporting_partial_coverage(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(ValueError):
                MODULE.merge([root / "missing.info"], root)

    def test_dedicated_campaign_requires_explicit_expected_count(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "crates/domain/src/lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text("one line\n")
            report = root / "full.info"
            report.write_text("SF:/runner/crates/domain/src/lib.rs\nDA:1,1\nend_of_record\n")
            with self.assertRaises(ValueError):
                MODULE.merge([report], root)
            result = MODULE.merge([report], root, expected_count=1)
            self.assertEqual(result["data"][0]["files"][0]["summary"]["lines"]["covered"], 1)
            with self.assertRaises(ValueError):
                MODULE.merge([report, report], root, expected_count=2)


if __name__ == "__main__":
    unittest.main()
