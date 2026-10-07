"""Focused validation for the native release entry point."""

import unittest
import tempfile
from pathlib import Path
from unittest.mock import patch

import build_release

from build_release import (artifact_snapshot, native_bundles, validate_built_artifacts,
                           validate_target_environment, validate_versions)


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

    def test_unchanged_and_duplicate_artifacts_cannot_pass_as_a_new_build(self):
        scratch = Path(__file__).resolve().parents[1] / ".scratch/p5f-w3-packaging"
        scratch.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=scratch) as temporary:
            root = Path(temporary)
            current = root / "FontCarver_0.1.0_amd64.deb"
            current.write_bytes(b"old candidate")
            before = artifact_snapshot(root, ["deb"])
            with self.assertRaisesRegex(ValueError, "unchanged"):
                validate_built_artifacts(root, ["deb"], before)
            current.write_bytes(b"new candidate with rebuilt bytes")
            validate_built_artifacts(root, ["deb"], before)
            stale = root / "FontCarver_0.0.1_amd64.deb"
            stale.write_bytes(b"stale")
            with self.assertRaisesRegex(ValueError, "found 2"):
                validate_built_artifacts(root, ["deb"], before)
            self.assertTrue(stale.exists(), "validation must not delete a user's old artifact")

    def test_source_fingerprint_includes_root_dependency_and_tooling_inputs(self):
        scratch = Path(__file__).resolve().parents[1] / ".scratch/p5f-w3-packaging"
        scratch.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=scratch) as temporary:
            root = Path(temporary)
            (root / "package.json").write_text('{"packageManager":"pnpm@10"}')
            lock = root / "pnpm-lock.yaml"
            lock.write_text("original lockfile")

            def git(arguments, **kwargs):
                if "ls-files" in arguments:
                    self.assertIn("package.json", arguments)
                    self.assertIn("pnpm-lock.yaml", arguments)
                    return b"package.json\0pnpm-lock.yaml\0"
                return "fixed-revision\n"

            with patch.object(build_release, "ROOT", root), patch.object(
                    build_release.subprocess, "check_output", side_effect=git):
                before = build_release.source_state()
                lock.write_text("changed dependency graph")
                after = build_release.source_state()
                self.assertNotEqual(before["input_sha256"], after["input_sha256"])
                self.assertEqual(after["input_files"], 2)

    def test_native_targets_are_explicit(self):
        self.assertEqual(native_bundles("Linux"), ["deb", "appimage"])
        self.assertEqual(native_bundles("Darwin"), ["dmg"])
        self.assertEqual(native_bundles("Windows"), ["msi"])
        with self.assertRaises(ValueError):
            native_bundles("FreeBSD")

    def test_thin_appimage_builder_is_gone(self):
        root = Path(build_release.__file__).resolve().parent
        self.assertFalse((root / "build_thin_appimage.py").exists())

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
