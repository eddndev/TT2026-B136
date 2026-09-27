"""Verify pinned media provisioning without network, compilers or native processes."""

import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("install_media_decoder", ROOT / "scripts/install_media_decoder.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

DEMUXERS = {"mov", "mp3", "wav", "image_jpeg_pipe", "image_png_pipe"}
DECODERS = {"h264", "aac", "mp3", "mp3float", "mjpeg", "png", "pcm_u8", "pcm_s16le", "pcm_s24le", "pcm_s32le"}
PARSERS = {"h264", "aac", "mpegaudio", "mjpeg", "png"}
FILTERS = {"null", "anull", "aformat", "format", "scale", "aresample"}


def capability_output(prefix, version="9.0.2", extra_protocol="", omitted_decoder=None):
    def output(command, **_kwargs):
        program = Path(command[0]).name
        option = command[-1]
        if option == "-version":
            return f"{program} version {version} Copyright FFmpeg developers\n"
        if option == "-buildconf":
            return "configuration:\n" + "\n".join(MODULE.configure_args(prefix)) + "\n"
        if option == "-protocols":
            return "Supported file protocols:\nInput:\n fd\n pipe\n" + extra_protocol + "\nOutput:\n pipe\n"
        if option == "-demuxers":
            return "Demuxers:\n D  mov,mp4,m4a,3gp,3g2,mj2 QuickTime\n D  mp3 MP3\n D  wav WAV\n D  jpeg_pipe JPEG\n D  png_pipe PNG\n"
        if option == "-decoders":
            return "Decoders:\n" + "".join(f" V..... {name} fixture\n" for name in sorted(DECODERS - {omitted_decoder}))
        if option == "-parsers":
            return "Parsers:\n" + "\n".join(sorted(PARSERS)) + "\n"
        if option == "-muxers":
            return "Muxers:\n  E null null output\n"
        if option == "-encoders":
            return "Encoders:\n V..... wrapped_avframe frame\n A..... pcm_s16le PCM\n"
        if option == "-filters":
            return "Filters:\n" + "".join(f" ... {name} fixture\n" for name in sorted(FILTERS))
        raise AssertionError(f"unexpected capability query: {option}")
    return output


def fake_binaries(prefix):
    (prefix / "bin").mkdir(parents=True)
    for name in ("ffmpeg", "ffprobe"):
        program = prefix / "bin" / name
        program.write_text("fixture executable, never run")
        program.chmod(0o700)


class TestInstallMediaDecoder(unittest.TestCase):
    def test_source_identity_and_closed_configuration_are_pinned(self):
        self.assertEqual(MODULE.VERSION, "9.0.2")
        self.assertEqual(MODULE.SOURCE_URL, "https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz")
        self.assertEqual(MODULE.SOURCE_SHA256, "8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e")
        self.assertEqual(MODULE.SIGNING_FINGERPRINT, "FCF986EA15E6E293A5644F10B4322F04D67658D8")
        prefix = Path("/opt/tt-media")
        args = MODULE.configure_args(prefix)
        for option in ("--disable-network", "--disable-autodetect", "--disable-everything",
                       "--disable-doc", "--disable-debug", "--disable-shared", "--enable-static",
                       "--enable-ffmpeg", "--enable-ffprobe", "--enable-zlib", "--enable-pthreads",
                       "--prefix=/opt/tt-media"):
            self.assertIn(option, args)
        for component, expected in [("protocol", {"fd", "pipe"}), ("demuxer", DEMUXERS),
                ("decoder", DECODERS), ("parser", PARSERS), ("filter", FILTERS),
                ("muxer", {"null"}), ("encoder", {"wrapped_avframe", "pcm_s16le"})]:
            options = [arg.split("=", 1)[1] for arg in args if arg.startswith(f"--enable-{component}=")]
            self.assertEqual(len(options), 1)
            self.assertEqual(set(options[0].split(",")), expected)

    def test_relative_prefix_is_rejected(self):
        with self.assertRaises(ValueError):
            MODULE.configure_args(Path("relative-prefix"))

    def test_bad_cached_digest_fails_before_extraction_or_network(self):
        with tempfile.TemporaryDirectory() as temporary:
            cache = Path(temporary)
            archive = cache / "ffmpeg-9.0.2.tar.xz"
            archive.write_bytes(b"altered source")
            with patch.object(MODULE.urllib.request, "urlopen") as network, patch.object(
                MODULE.tarfile, "open"
            ) as extract, self.assertRaisesRegex(RuntimeError, "digest|SHA"):
                MODULE.source_archive(cache)
            network.assert_not_called()
            extract.assert_not_called()
            self.assertEqual(archive.read_bytes(), b"altered source")

    def test_verified_cache_is_reused_without_download(self):
        with tempfile.TemporaryDirectory() as temporary:
            cache = Path(temporary)
            archive = cache / "ffmpeg-9.0.2.tar.xz"
            archive.write_bytes(b"verified fixture")
            with patch.object(MODULE, "SOURCE_SHA256", hashlib.sha256(archive.read_bytes()).hexdigest()), patch.object(
                MODULE.urllib.request, "urlopen"
            ) as network:
                self.assertEqual(MODULE.source_archive(cache), archive)
            network.assert_not_called()

    def test_extraction_refuses_escape_members_even_with_matching_digest(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "malicious.tar.xz"
            with tarfile.open(archive, "w:xz") as stream:
                member = tarfile.TarInfo("../../outside")
                member.size = 1
                stream.addfile(member, io.BytesIO(b"x"))
            with patch.object(MODULE, "SOURCE_SHA256", hashlib.sha256(archive.read_bytes()).hexdigest()), self.assertRaises(
                (RuntimeError, ValueError, tarfile.TarError)
            ):
                MODULE.extract_source(archive, root / "build")
            self.assertFalse((root / "outside").exists())

    def test_capability_check_rejects_wrong_version_network_and_missing_decoder(self):
        with tempfile.TemporaryDirectory() as temporary:
            prefix = Path(temporary) / "installed"
            fake_binaries(prefix)
            for options in ({"version": "8.0"}, {"extra_protocol": " http"}, {"omitted_decoder": "h264"}):
                with patch.object(MODULE.subprocess, "check_output", side_effect=capability_output(prefix, **options)), self.assertRaises(RuntimeError):
                    MODULE.verify_binaries(prefix)
            with patch.object(MODULE.subprocess, "check_output", side_effect=capability_output(prefix)):
                MODULE.verify_binaries(prefix)

    def test_failed_capabilities_leave_no_completed_install_and_use_one_compiler(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            prefix, cache, source = root / "installed", root / "cache", root / "source"
            source.mkdir()
            calls = []

            def execute(command, **_kwargs):
                calls.append(command)
                if command[0] == "make" and "install" in command:
                    destination = next(value.split("=", 1)[1] for value in command if value.startswith("DESTDIR="))
                    fake_binaries(Path(destination) / prefix.relative_to("/"))

            with patch.object(MODULE, "source_archive", return_value=root / "source.tar.xz"), patch.object(
                MODULE, "extract_source", return_value=source
            ), patch.object(MODULE.subprocess, "run", side_effect=execute), patch.object(
                MODULE, "verify_binaries", side_effect=RuntimeError("missing decoder")
            ), self.assertRaisesRegex(RuntimeError, "missing decoder"):
                MODULE.install(prefix, cache)
            self.assertIn(["make", "-j1"], calls)
            self.assertFalse(prefix.exists())
            self.assertFalse(any(root.rglob(".tt-media-decoder.json")))

    def test_existing_wrong_version_is_never_rebuilt_or_overwritten(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            prefix = root / "installed"
            fake_binaries(prefix)
            marker = prefix / ".tt-media-decoder.json"
            marker.write_text(json.dumps(MODULE.manifest(prefix)))
            original = (prefix / "bin" / "ffmpeg").read_bytes()
            with patch.object(MODULE, "source_archive") as fetch, patch.object(
                MODULE.subprocess, "check_output", side_effect=capability_output(prefix, version="8.0")
            ), self.assertRaises(RuntimeError):
                MODULE.install(prefix, root / "cache")
            fetch.assert_not_called()
            self.assertEqual((prefix / "bin" / "ffmpeg").read_bytes(), original)
            self.assertTrue(marker.exists())


    def test_verify_cli_needs_no_cache_and_never_writes_or_locks(self):
        with tempfile.TemporaryDirectory() as temporary:
            prefix = Path(temporary) / "installed"
            fake_binaries(prefix)
            (prefix / ".tt-media-decoder.json").write_text(json.dumps(MODULE.manifest(prefix)))
            original_open = Path.open

            def read_only_open(path, mode="r", *args, **kwargs):
                if any(flag in mode for flag in "wax+"):
                    raise AssertionError("verification attempted to open a file for writing")
                return original_open(path, mode, *args, **kwargs)

            with patch("sys.argv", ["install_media_decoder.py", "--verify", "--prefix", str(prefix)]), patch.object(
                Path, "open", read_only_open
            ), patch.object(Path, "mkdir", side_effect=AssertionError("verification attempted mkdir")), patch.object(
                MODULE.fcntl, "flock", side_effect=AssertionError("verification attempted locking")
            ), patch.object(MODULE, "source_archive", side_effect=AssertionError("verification fetched sources")), patch.object(
                MODULE.subprocess, "run", side_effect=AssertionError("verification attempted a build")
            ), patch.object(MODULE.subprocess, "check_output", side_effect=capability_output(prefix)), patch("builtins.print"):
                MODULE.main()

    def test_read_only_verification_rejects_mismatched_marker_before_processes(self):
        with tempfile.TemporaryDirectory() as temporary:
            prefix = Path(temporary) / "installed"
            fake_binaries(prefix)
            marker = MODULE.manifest(prefix)
            marker["sha256"] = "0" * 64
            (prefix / ".tt-media-decoder.json").write_text(json.dumps(marker))
            with patch.object(MODULE.subprocess, "check_output") as native, self.assertRaisesRegex(RuntimeError, "marker"):
                MODULE.verify_install(prefix)
            native.assert_not_called()

    def test_install_cli_still_requires_explicit_cache(self):
        with patch("sys.argv", ["install_media_decoder.py", "--prefix", "/opt/tt-media"]), patch.object(
            MODULE, "install"
        ) as install, patch("sys.stderr", io.StringIO()), self.assertRaises(SystemExit) as raised:
            MODULE.main()
        self.assertEqual(raised.exception.code, 2)
        install.assert_not_called()


    def test_ffmpeg_exact_terminal_help_footer_preserves_configuration(self):
        with tempfile.TemporaryDirectory() as temporary:
            prefix = Path(temporary) / "installed"
            fake_binaries(prefix)
            clean_output = capability_output(prefix)

            def actual_cli_output(command, **kwargs):
                output = clean_output(command, **kwargs)
                if Path(command[0]).name == "ffmpeg" and command[-1] == "-buildconf":
                    output += "Exiting with exit code 0\n"
                return output

            with patch.object(MODULE.subprocess, "check_output", side_effect=actual_cli_output):
                MODULE.verify_binaries(prefix)

    def test_footer_handling_still_rejects_unexpected_configuration_tokens(self):
        with tempfile.TemporaryDirectory() as temporary:
            prefix = Path(temporary) / "installed"
            fake_binaries(prefix)
            clean_output = capability_output(prefix)

            def unexpected_configuration(command, **kwargs):
                output = clean_output(command, **kwargs)
                if command[-1] == "-buildconf":
                    output += "--enable-network\nExiting with exit code 0\n"
                return output

            with patch.object(MODULE.subprocess, "check_output", side_effect=unexpected_configuration), self.assertRaisesRegex(
                RuntimeError, "configuration"
            ):
                MODULE.verify_binaries(prefix)

    def test_footer_exception_is_only_exact_terminal_zero_for_ffmpeg(self):
        with tempfile.TemporaryDirectory() as temporary:
            prefix = Path(temporary) / "installed"
            fake_binaries(prefix)
            clean_output = capability_output(prefix)
            for program, footer, misplaced in [
                ("ffmpeg", "Exiting with exit code 7\n", False),
                ("ffmpeg", "Exiting with exit code 0\n", True),
                ("ffmpeg", "Exiting with exit code 0 \n", False),
                ("ffmpeg", "Exiting with exit code 0\r\n", False),
                ("ffprobe", "Exiting with exit code 0\n", False),
            ]:
                def malformed_footer(command, **kwargs):
                    output = clean_output(command, **kwargs)
                    if Path(command[0]).name == program and command[-1] == "-buildconf":
                        if misplaced:
                            return output.replace("configuration:\n", "configuration:\n" + footer, 1)
                        return output + footer
                    return output

                with self.subTest(program=program, footer=footer, misplaced=misplaced), patch.object(
                    MODULE.subprocess, "check_output", side_effect=malformed_footer
                ), self.assertRaisesRegex(RuntimeError, "configuration"):
                    MODULE.verify_binaries(prefix)

    def test_success_footer_cannot_mask_a_failed_native_process(self):
        with tempfile.TemporaryDirectory() as temporary:
            prefix = Path(temporary) / "installed"
            fake_binaries(prefix)
            failure = MODULE.subprocess.CalledProcessError(7, ["ffmpeg"], output="Exiting with exit code 0\n")
            with patch.object(MODULE.subprocess, "check_output", side_effect=failure), self.assertRaises(
                MODULE.subprocess.CalledProcessError
            ):
                MODULE.verify_binaries(prefix)


if __name__ == "__main__":
    unittest.main()
