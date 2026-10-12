"""Release policy checks fail closed and retain upstream attribution."""

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import check_release_licenses as licenses


class LicensePolicyTests(unittest.TestCase):
    def test_permissive_or_branch_can_be_chosen(self):
        for expression in [
            "MIT OR GPL-3.0-only",
            "MIT/Apache-2.0",
            "(MIT OR Apache-2.0) AND Unicode-3.0",
            "Apache-2.0 WITH LLVM-exception",
        ]:
            self.assertTrue(licenses.allowed_expression(expression, set()), expression)

    def test_all_and_obligations_must_be_allowed(self):
        for expression in [
            "MIT AND GPL-3.0-only",
            "(MIT OR BSD-3-Clause) AND LGPL-2.1-only",
            "MIT AND (GPL-3.0-only OR MPL-2.0)",
        ]:
            self.assertFalse(licenses.allowed_expression(expression, set()), expression)

    def test_missing_unknown_and_malformed_licenses_fail_closed(self):
        for expression in [None, "", "MIT OR", "MIT AND", "(MIT", "MIT)",
                           "MIT OR OR GPL-3.0-only", "MIT frobnicate GPL-3.0-only",
                           "MIT WITH Unreviewed-exception", "MIT OR ???",
                           "SEE LICENSE IN LICENSE"]:
            self.assertFalse(licenses.allowed_expression(expression, set()), expression)

    def test_exception_is_bound_to_package_version_and_documented_ruling(self):
        notice = "Five crates are unmodified. **Ruled acceptable.**\n" \
                 "| `cssparser` | 0.36.0 | via tauri |"
        self.assertEqual(licenses.exceptions_for("cargo", "cssparser", "0.36.0", notice),
                         {"MPL-2.0"})
        self.assertEqual(licenses.exceptions_for("cargo", "cssparser", "0.37.0", notice), set())
        self.assertEqual(licenses.exceptions_for("cargo", "cssparser", "0.36.0", ""), set())
        self.assertFalse(licenses.allowed_expression("MPL-2.0", set()))

    def test_npm_exception_does_not_allow_other_packages_or_versions(self):
        notice = "`minimatch` 10.2.6 — **BlueOak-1.0.0**. Acceptable."
        self.assertEqual(licenses.exceptions_for("npm", "minimatch", "10.2.6", notice),
                         {"BlueOak-1.0.0"})
        self.assertEqual(licenses.exceptions_for("npm", "minimatch", "11.0.0", notice), set())
        self.assertEqual(licenses.exceptions_for("npm", "other", "10.2.6", notice), set())


class NoticeTests(unittest.TestCase):
    def test_pnpm_aggregated_versions_are_paired_using_installed_manifests(self):
        with tempfile.TemporaryDirectory() as directory:
            roots = []
            for version in ("1.0.0", "2.0.0"):
                root = Path(directory) / version
                root.mkdir()
                (root / "package.json").write_text(json.dumps({
                    "name": "example", "version": version, "license": "MIT",
                }))
                roots.append(str(root))
            inventory = {"MIT": [{"name": "example", "versions": ["2.0.0", "1.0.0"],
                                   "paths": roots, "license": "MIT"}]}
            self.assertEqual({item.version for item in licenses.npm_dependencies(inventory)},
                             {"1.0.0", "2.0.0"})
            inventory["MIT"][0]["paths"].pop()
            with self.assertRaisesRegex(ValueError, "uninstalled versions"):
                licenses.npm_dependencies(inventory)

    def test_modified_fallback_notice_fails_its_pinned_hash(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            catalog_root = root / "scripts/release_licenses"
            catalog_root.mkdir(parents=True)
            (catalog_root / "LICENSE").write_text("changed")
            (catalog_root / "npm.json").write_text(json.dumps({"example@1.0.0": [{
                "path": "LICENSE", "sha256": hashlib.sha256(b"original").hexdigest(),
                "source": "https://example.com/pinned/LICENSE",
            }]}))
            item = licenses.Dependency("npm", "example", "1.0.0", "MIT", root, "")
            with self.assertRaisesRegex(ValueError, "hash mismatch"):
                licenses.supplemental_notices(root, item)

    def test_collects_license_and_notice_text_without_following_external_links(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "package"
            package.mkdir()
            (package / "LICENSE-MIT").write_text("copyright and permission")
            (package / "NOTICE").write_text("upstream attribution")
            (root / "secret").write_text("not a notice")
            (package / "LICENSE-link").symlink_to(root / "secret")
            self.assertEqual({p.name for p in licenses.notice_files(package)},
                             {"LICENSE-MIT", "NOTICE"})

    def test_missing_notices_are_detected(self):
        with tempfile.TemporaryDirectory() as directory:
            self.assertEqual(licenses.notice_files(Path(directory)), [])
            root = Path(directory)
            (root / "LICENSE").write_text("")
            self.assertEqual(licenses.notice_files(root), [])
            item = licenses.Dependency("npm", "example", "1.0.0", "MIT", root, "")
            with self.assertRaisesRegex(ValueError, "missing upstream notice"):
                licenses.write_bundle(root, root / "out", [item])
            self.assertFalse((root / "out").exists())

    def test_notice_bundle_retains_exact_text_and_records_hash(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "package"
            package.mkdir()
            upstream = package / "LICENSE"
            upstream.write_bytes(b"Copyright 2026 Test author\nPermission granted.\n")
            for name in licenses.PROJECT_NOTICES:
                source = root / name
                source.parent.mkdir(parents=True, exist_ok=True)
                source.write_text("Project notice " + name)
            item = licenses.Dependency("npm", "example", "1.0.0", "MIT", package,
                                       "https://example.com/repo")
            result = licenses.write_bundle(root, root / "out", [item])
            copied = root / "out" / result[0]["notices"][0]["path"]
            self.assertEqual(copied.read_bytes(), upstream.read_bytes())
            self.assertEqual(len(result[0]["notices"][0]["sha256"]), 64)
            self.assertTrue((root / "out" / "DEPENDENCIES.json").is_file())


if __name__ == "__main__":
    unittest.main()
