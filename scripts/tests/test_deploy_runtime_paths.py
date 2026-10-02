"""The selected release supplies every native upload decoder."""
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
import runtime


class RuntimePathTests(unittest.TestCase):
    def test_api_selects_both_media_executables_from_the_current_release(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "releases" / "v1.0.0"
            target.mkdir(parents=True)
            (root / "current").symlink_to(target)
            with patch.object(runtime, "settings", return_value={"api_port": 18087}), \
                    patch.object(runtime, "environment", return_value={}), \
                    patch.object(runtime.os, "chdir"), \
                    patch.object(runtime.os, "execve") as execute:
                runtime.serve(root)
            executable, arguments, environment = execute.call_args.args
            self.assertEqual(executable, str(target / "bin/despacho-cli"))
            self.assertIn("--ffprobe-path", arguments)
            self.assertIn("--ffmpeg-path", arguments)
            self.assertEqual(arguments[arguments.index("--ffprobe-path") + 1],
                             str(target / "bin/ffprobe"))
            self.assertEqual(arguments[arguments.index("--ffmpeg-path") + 1],
                             str(target / "bin/ffmpeg"))
            self.assertEqual(environment["LD_LIBRARY_PATH"], str(target / "lib"))


if __name__ == "__main__":
    unittest.main()
