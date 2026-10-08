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
        self.assertEqual(native_bundles("Linux"), ["deb", "rpm", "appimage"])
        self.assertEqual(native_bundles("Darwin"), ["dmg"])
        self.assertEqual(native_bundles("Windows"), ["nsis"])
        with self.assertRaises(ValueError):
            native_bundles("FreeBSD")

    def test_thin_appimage_builder_is_gone(self):
        root = Path(build_release.__file__).resolve().parent
        self.assertFalse((root / "build_thin_appimage.py").exists())

    def test_different_versions_cannot_ship(self):
        with self.assertRaisesRegex(ValueError, "version mismatch"):
            validate_versions("0.1.0", "0.2.0", "0.1.0")

    def test_version_must_be_encodable_by_native_installers(self):
        for invalid in ["0.1.0-dev", "256.1.0", "1.256.0", "1.0.65536", "1.0"]:
            with self.subTest(version=invalid):
                with self.assertRaises(ValueError):
                    validate_versions(invalid, invalid, invalid)
        self.assertEqual(validate_versions("0.1.0", "0.1.0", "0.1.0"), "0.1.0")


    def test_one_artifact_is_expected_per_format(self):
        import inspect
        # msi_languages defaulted to 2; a build is correct with one artifact per
        # format now, so the parameter must be gone, not merely unused.
        self.assertNotIn("msi_languages",
                         inspect.signature(validate_built_artifacts).parameters)
        with tempfile.TemporaryDirectory() as directory:
            bundle = Path(directory)
            for kind in ("nsis", "rpm"):
                (bundle / kind).mkdir()
            (bundle / "nsis/FontCarver_0.1.0_x64-setup.exe").write_bytes(b"MZ")
            (bundle / "rpm/FontCarver-0.1.0-1.x86_64.rpm").write_bytes(b"\xed\xab\xee\xdb")
            validate_built_artifacts(bundle, ["nsis", "rpm"], {})
            (bundle / "nsis/FontCarver_0.1.0_x86-setup.exe").write_bytes(b"MZ")
            with self.assertRaisesRegex(ValueError, "nsis"):
                validate_built_artifacts(bundle, ["nsis", "rpm"], {})

    def test_bundle_configuration_declares_rpm_and_nsis(self):
        import json
        config = json.loads((Path(build_release.__file__).resolve().parents[1]
                             / "src-tauri/tauri.conf.json").read_text())
        self.assertIn("rpm", config["bundle"]["linux"])
        self.assertIn("nsis", config["bundle"]["windows"])
        self.assertNotIn("wix", config["bundle"]["windows"])
        self.assertEqual(config["bundle"]["windows"]["nsis"]["languages"],
                         ["English", "SimpChinese"])


    def _portable_inputs(self, work):
        """A built executable plus the licenses directory the release helper generates."""
        import json
        import check_release_bundle as verifier
        exe = work / "fontcarver-desktop.exe"
        exe.write_bytes(b"MZ" + b"\0" * 64)
        licenses = work / "licenses"
        licenses.mkdir()
        for name in ("LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY.md"):
            (licenses / name).write_text(name)
        notice = licenses / "licenses" / "dependency.txt"
        notice.parent.mkdir()
        notice.write_text("Permission to redistribute.\n")
        (licenses / "DEPENDENCIES.json").write_text(json.dumps({"schema_version": 1, "packages": [
            {"ecosystem": "cargo", "name": "example", "version": "1.0", "license": "MIT",
             "notices": [{"path": "licenses/dependency.txt", "sha256": verifier.sha256(notice)}]}]}))
        return exe, licenses

    def test_portable_zip_layout_passes_the_payload_inspector(self):
        # A round trip, because the inspector finds licenses by sibling layout and a
        # zip that merely contains the right files in the wrong folders is rejected.
        import zipfile
        import check_release_bundle as verifier
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            exe, licenses = self._portable_inputs(work)
            archive = build_release.build_portable_zip(exe, licenses, "0.1.0", work / "out")
            self.assertEqual(archive.name, "FontCarver_0.1.0_x64-portable.zip")
            with zipfile.ZipFile(archive) as zip_file:
                self.assertEqual({name.split("/", 1)[0] for name in zip_file.namelist()},
                                 {"FontCarver-0.1.0"})
                zip_file.extractall(work / "x")
            report = verifier.inspect_payload(work / "x/FontCarver-0.1.0", "portable")
            self.assertEqual(report["executable"]["paths"], ["fontcarver-desktop.exe"])

    def test_portable_payload_without_its_marker_is_rejected(self):
        import zipfile
        import check_release_bundle as verifier
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            exe, licenses = self._portable_inputs(work)
            archive = build_release.build_portable_zip(exe, licenses, "0.1.0", work / "out")
            with zipfile.ZipFile(archive) as zip_file:
                zip_file.extractall(work / "x")
            (work / "x/FontCarver-0.1.0/PORTABLE.txt").unlink()
            with self.assertRaisesRegex(verifier.VerificationError, "PORTABLE.txt"):
                verifier.inspect_payload(work / "x/FontCarver-0.1.0", "portable")


if __name__ == "__main__":
    unittest.main()
