from pathlib import Path
import plistlib
import tempfile
import unittest

from native_baseline_bundle import prepare


class NativeBundleTests(unittest.TestCase):
    def test_launch_identity_uses_binary_and_preserves_explicit_fixture_paths(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            binary = root / "recorder"
            binary.write_bytes(b"test executable")
            binary.chmod(0o755)
            output = root / "local.jsonl"
            bundle = prepare(binary, "local", output, root / "Local.app")
            with (bundle / "Contents/Info.plist").open("rb") as stream:
                info = plistlib.load(stream)
            executable = bundle / "Contents/MacOS" / info["CFBundleExecutable"]
            self.assertEqual(executable.read_bytes(), binary.read_bytes())
            self.assertTrue(executable.stat().st_mode & 0o111)
            self.assertEqual(info["LSEnvironment"], {
                "RADIANT_BASELINE_MODE": "local",
                "RADIANT_BASELINE_OUTPUT": str(output),
            })
            self.assertFalse(output.exists())
            with self.assertRaises(FileExistsError):
                prepare(binary, "local", output, bundle)

    def test_existing_raw_output_and_invalid_mode_are_preserved(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            binary = root / "recorder"
            binary.write_bytes(b"test executable")
            output = root / "raw.jsonl"
            output.write_bytes(b"original observations")
            for mode in ("local", "unknown"):
                with self.assertRaises(ValueError):
                    prepare(binary, mode, output, root / "New.app")
                self.assertFalse((root / "New.app").exists())
            self.assertEqual(output.read_bytes(), b"original observations")


if __name__ == "__main__":
    unittest.main()
