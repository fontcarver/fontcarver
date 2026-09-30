"""Focused validation for the native release entry point."""

import unittest
import tempfile
from pathlib import Path

from build_release import native_bundles, validate_target_environment, validate_versions


class ReleaseValidationTests(unittest.TestCase):
    def test_custom_build_target_cannot_select_stale_native_artifacts(self):
        with self.assertRaisesRegex(ValueError, "CARGO_BUILD_TARGET"):
            validate_target_environment({"CARGO_BUILD_TARGET": "aarch64-unknown-linux-gnu"})
        validate_target_environment({})

    def test_cargo_configuration_target_cannot_change_bundle_output_layout(self):
        scratch = Path(__file__).resolve().parents[1] / ".scratch/p5-release-tests"
        scratch.mkdir(parents=True, exist_ok=True)
        try:
            with tempfile.TemporaryDirectory(dir=scratch) as temporary:
                config = Path(temporary) / "config.toml"
                config.write_text('[build]\ntarget = "aarch64-unknown-linux-gnu"\n', encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "build.target"):
                    validate_target_environment({}, [config])
        finally:
            scratch.rmdir()

    def test_native_targets_are_explicit(self):
        self.assertEqual(native_bundles("Linux"), ["deb", "appimage"])
        self.assertEqual(native_bundles("Darwin"), ["dmg"])
        self.assertEqual(native_bundles("Windows"), ["msi"])
        with self.assertRaises(ValueError):
            native_bundles("FreeBSD")

    def test_different_versions_cannot_ship(self):
        with self.assertRaisesRegex(ValueError, "version mismatch"):
            validate_versions("0.1.0", "0.2.0", "0.1.0")

    def test_version_must_be_accepted_by_msi(self):
        for invalid in ["0.1.0-dev", "256.1.0", "1.256.0", "1.0.65536", "1.0"]:
            with self.subTest(version=invalid):
                with self.assertRaises(ValueError):
                    validate_versions(invalid, invalid, invalid)
        self.assertEqual(validate_versions("0.1.0", "0.1.0", "0.1.0"), "0.1.0")


if __name__ == "__main__":
    unittest.main()
