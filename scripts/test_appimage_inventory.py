"""Native payload evidence must not imply unearned redistribution approval."""

import hashlib
import json
from pathlib import Path
import struct
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import appimage_inventory as inventory
import appimage_policy


def elf(machine=62):
    return b'\x7fELF\x02\x01' + bytes(12) + struct.pack('<H', machine) + bytes(44)


class InventoryTests(unittest.TestCase):
    def test_gate_never_approves_current_appimage_toolchain(self):
        with self.assertRaisesRegex(ValueError, 'LGPL libfuse'):
            appimage_policy.require_releasable_appimage()

    def test_missing_license_is_unknown_not_permissive(self):
        self.assertEqual(inventory.license_summary('No machine readable fields'),
                         {'labels': [], 'copyleft_mentioned': False,
                          'classification': 'unknown'})

    def test_package_license_is_a_review_hint_not_file_ruling(self):
        result = inventory.license_summary('License: MIT\nLicense: LGPL-2.1+\n')
        self.assertTrue(result['copyleft_mentioned'])
        self.assertEqual(result['classification'], 'needs-file-review')
        self.assertEqual(result['labels'], ['LGPL-2.1+', 'MIT'])
        self.assertEqual(inventory.license_summary('License: MIT\n')['classification'],
                         'needs-file-review')

    def test_inventory_accounts_for_regular_files_directories_and_symlinks(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'usr').mkdir()
            (root / 'usr/library.so').write_bytes(elf())
            (root / 'usr/link.so').symlink_to('library.so')
            (root / 'AppRun').write_text('#!/bin/sh\n')
            rows = inventory.payload_files(root)
            self.assertEqual(len(rows), 4)
            self.assertEqual({row['type'] for row in rows}, {'directory', 'file', 'symlink'})
            library = next(row for row in rows if row['path'] == 'usr/library.so')
            self.assertEqual(library['elf_machine'], 62)
            self.assertEqual(len(library['sha256']), 64)
            self.assertEqual(next(row for row in rows if row['type'] == 'symlink')['target'],
                             'library.so')

    def test_external_and_broken_symlinks_fail_without_reading_target(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'payload'
            root.mkdir()
            (root / 'outside').symlink_to('../outside')
            with self.assertRaisesRegex(ValueError, 'unsafe or broken symlink'):
                inventory.payload_files(root)
            (root / 'outside').unlink()
            (root / 'broken').symlink_to('missing')
            with self.assertRaisesRegex(ValueError, 'unsafe or broken symlink'):
                inventory.payload_files(root)

    def test_runtime_is_separate_and_offset_must_point_to_squashfs(self):
        with tempfile.TemporaryDirectory() as directory:
            artifact = Path(directory) / 'test.AppImage'
            artifact.write_bytes(elf() + b'hsqs' + bytes(40))
            result = inventory.runtime_record(artifact, 64)
            self.assertEqual(result['bytes'], 64)
            self.assertEqual(result['license_status'], 'unreviewed')
            with self.assertRaisesRegex(ValueError, 'SquashFS'):
                inventory.runtime_record(artifact, 63)
            with self.assertRaisesRegex(ValueError, 'offset'):
                inventory.runtime_record(artifact, -1)

    def test_host_candidates_match_architecture_and_do_not_claim_build_provenance(self):
        with tempfile.TemporaryDirectory() as directory:
            host = Path(directory)
            (host / 'usr/lib').mkdir(parents=True)
            (host / 'usr/lib/libtest.so').write_bytes(elf())
            rows = [{'path': 'usr/lib/libtest.so', 'type': 'file',
                     'elf_machine': 62, 'sha256': 'different'}]
            packages = {'test:amd64': {'version': '1', 'source_package': 'test',
                                      'source_version': '1'}}
            owners = {'libtest.so': [('test:amd64', '/usr/lib/libtest.so')]}
            inventory.annotate_host_candidates(rows, owners, packages, host)
            candidate = rows[0]['host_candidates'][0]
            self.assertEqual(candidate['match'], 'name-and-architecture-only')
            self.assertFalse(candidate['content_identical'])


class ThinPolicyTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / 'payload'
        self.evidence = self.root / 'usr/lib/FontCarver/licenses/thin-appimage'
        self.evidence.mkdir(parents=True)
        self.binary = self.root / 'usr/bin/fontcarver-desktop'
        self.binary.parent.mkdir(parents=True)
        self.binary.write_bytes(elf() + b'__TAURI_BUNDLE_TYPE_VAR_APP\0')
        self.launcher = self.root / 'AppRun'
        self.launcher.write_bytes(b'#!/bin/sh\nexec "$APPDIR/usr/bin/fontcarver-desktop" "$@"\n')
        self.launcher.chmod(0o755)
        (self.evidence / 'runtime.c').write_bytes(b'audited MIT source')
        self.link_map = self.evidence / 'runtime.map'
        self.link_map.write_text('LOAD libfuse3.so\nLOAD libsquashfuse.so\nLOAD libsquashfuse_ll.so\n')
        self.needed = {'libfuse3.so.3', 'libsquashfuse.so.0', 'libsquashfuse_ll.so.0', 'libc.so.6'}
        self.recipe = SimpleNamespace(
            PROFILE='fontcarver-host-libs-v1', COMMIT='approved-commit', APP_RUN=self.launcher.read_bytes(),
            REQUIRED_NEEDED=self.needed,
            APPROVED_INPUTS={'runtime.c': hashlib.sha256(b'audited MIT source').hexdigest()})
        self.artifact = Path(self.temporary.name) / 'thin.AppImage'
        self.artifact.write_bytes(elf() + b'hsqs' + bytes(40))
        self.receipt = {'schema_version': 1, 'profile': self.recipe.PROFILE,
                        'source_commit': self.recipe.COMMIT,
                        'inputs': {**self.recipe.APPROVED_INPUTS,
                                   'runtime.map': hashlib.sha256(self.link_map.read_bytes()).hexdigest()},
                        'apprun_sha256': hashlib.sha256(self.recipe.APP_RUN).hexdigest(),
                        'application': {
                            'source_sha256': hashlib.sha256(elf() + b'__TAURI_BUNDLE_TYPE_VAR_DEB\0').hexdigest(),
                            'appimage_sha256': hashlib.sha256(self.binary.read_bytes()).hexdigest(),
                            'marker_offset': len(elf()) + len(b'__TAURI_BUNDLE_TYPE_VAR_'),
                            'before': 'DEB', 'after': 'APP'},
                        'runtime': {'bytes': 64, 'sha256': hashlib.sha256(elf()).hexdigest(),
                                    'needed': sorted(self.needed)}}
        self.write_receipt()
        readelf = '\n'.join(f'(NEEDED) Shared library: [{name}]' for name in self.needed)
        self.readelf = self.enterContext(patch.object(
            appimage_policy.subprocess, 'run', return_value=SimpleNamespace(stdout=readelf)))
        self.enterContext(patch.object(appimage_policy, '_approved_recipe', return_value=self.recipe))

    def write_receipt(self):
        (self.evidence / 'BUILD.json').write_text(json.dumps(self.receipt))

    def verify(self):
        return appimage_policy.require_releasable_appimage(self.root, self.artifact)

    def test_accepts_only_validated_recipe_evidence(self):
        self.assertEqual(self.verify()['bundled_shared_libraries'], 0)

    def test_missing_receipt_blocks_original_images(self):
        (self.evidence / 'BUILD.json').unlink()
        with self.assertRaisesRegex(ValueError, 'missing audited'):
            self.verify()

    def test_actual_application_marker_must_be_unique_appimage(self):
        prefix = b'__TAURI_BUNDLE_TYPE_VAR_'
        for markers in (b'', prefix + b'DEB', prefix + b'UNK',
                        prefix + b'APP' + prefix + b'APP', prefix + b'APP' + prefix + b'DEB'):
            with self.subTest(markers=markers):
                self.binary.write_bytes(elf() + markers)
                with self.assertRaisesRegex(ValueError, 'application'):
                    self.verify()

    def test_application_conversion_receipt_is_required(self):
        for value in (None, [], {}):
            with self.subTest(value=value):
                self.receipt['application'] = value
                self.write_receipt()
                with self.assertRaisesRegex(ValueError, 'application'):
                    self.verify()

    def test_application_conversion_claims_must_match_bytes(self):
        original = dict(self.receipt['application'])
        for field, value in (('source_sha256', '0' * 64), ('appimage_sha256', '0' * 64),
                             ('marker_offset', 0), ('marker_offset', True),
                             ('before', 'UNK'), ('after', 'DEB')):
            with self.subTest(field=field, value=value):
                self.receipt['application'] = {**original, field: value}
                self.write_receipt()
                with self.assertRaisesRegex(ValueError, 'application'):
                    self.verify()

    def test_accepted_application_conversion_records_source_and_three_changed_bytes(self):
        proof = self.verify()['application']
        self.assertEqual(proof['source_sha256'], self.receipt['application']['source_sha256'])
        self.assertEqual(proof['changed_bytes'], 3)
        self.assertEqual(proof['bundle_type'], 'APP')

    def test_receipt_approval_cannot_cover_changed_runtime(self):
        self.artifact.write_bytes(elf() + b'hsqs' + bytes(40))
        self.receipt['runtime']['sha256'] = '0' * 64
        self.receipt['release_eligible'] = True
        self.write_receipt()
        with self.assertRaisesRegex(ValueError, 'prefix hash mismatch'):
            self.verify()

    def test_original_source_and_license_bytes_are_pinned(self):
        (self.evidence / 'runtime.c').write_text('different source')
        with self.assertRaisesRegex(ValueError, 'pin mismatch'):
            self.verify()

    def test_bundled_library_is_rejected_even_when_receipt_claims_clean(self):
        (self.root / 'libfuse3.so.3').write_bytes(elf())
        with self.assertRaisesRegex(ValueError, 'bundles a shared library'):
            self.verify()

    def test_unapproved_executable_cannot_hide_without_so_extension(self):
        (self.root / 'extra').write_bytes(elf())
        with self.assertRaisesRegex(ValueError, 'only the FontCarver ELF'):
            self.verify()

    def test_static_fuse_or_missing_dynamic_dependency_is_rejected(self):
        self.link_map.write_text(self.link_map.read_text() + 'LOAD libfuse3.a\n')
        self.receipt['inputs']['runtime.map'] = hashlib.sha256(self.link_map.read_bytes()).hexdigest()
        self.write_receipt()
        with self.assertRaisesRegex(ValueError, 'forbidden static library'):
            self.verify()
        self.link_map.write_text('LOAD libfuse3.so\nLOAD libsquashfuse.so\nLOAD libsquashfuse_ll.so\n')
        self.receipt['inputs']['runtime.map'] = hashlib.sha256(self.link_map.read_bytes()).hexdigest()
        self.write_receipt()
        self.readelf.return_value.stdout = '(NEEDED) Shared library: [libc.so.6]'
        with self.assertRaisesRegex(ValueError, 'approved host shared libraries'):
            self.verify()

    def test_approved_recipe_cannot_redirect_dynamic_library_resolution(self):
        self.readelf.return_value.stdout += '\n(RUNPATH) Library runpath: [/private/libraries]'
        with self.assertRaisesRegex(ValueError, 'overrides host library search paths'):
            self.verify()

    def test_other_static_archives_and_extracted_compiler_members_are_rejected(self):
        original = self.link_map.read_text()
        for extra in ('LOAD libzstd.a\n', 'libc_nonshared.a(atexit.o)\n'):
            with self.subTest(extra=extra):
                self.link_map.write_text(original + extra)
                self.receipt['inputs']['runtime.map'] = hashlib.sha256(self.link_map.read_bytes()).hexdigest()
                self.write_receipt()
                with self.assertRaisesRegex(ValueError, 'forbidden static library'):
                    self.verify()

    def test_launcher_cannot_override_system_library_loading(self):
        self.launcher.write_text('#!/bin/sh\nexport LD_LIBRARY_PATH="$APPDIR/lib"\n')
        with self.assertRaisesRegex(ValueError, 'approved host-library launcher'):
            self.verify()

    def test_malformed_receipts_fail_with_actionable_validation_errors(self):
        original = json.dumps(self.receipt)
        for malformed in (None, [], {'schema_version': 1, 'profile': self.recipe.PROFILE,
                                    'apprun_sha256': self.receipt['apprun_sha256'], 'runtime': []}):
            with self.subTest(malformed=malformed):
                self.receipt = malformed
                self.write_receipt()
                with self.assertRaises(ValueError):
                    self.verify()
        for field, value in (('needed', [None]), ('needed', None), ('bytes', True)):
            with self.subTest(field=field, value=value):
                self.receipt = json.loads(original)
                self.receipt['runtime'][field] = value
                self.write_receipt()
                with self.assertRaises(ValueError):
                    self.verify()


if __name__ == '__main__':
    unittest.main()
