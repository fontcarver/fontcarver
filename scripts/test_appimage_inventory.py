"""Native payload evidence must not imply unearned redistribution approval."""

from pathlib import Path
import struct
import tempfile
import unittest

import appimage_inventory as inventory


def elf(machine=62):
    return b'\x7fELF\x02\x01' + bytes(12) + struct.pack('<H', machine) + bytes(44)


class InventoryTests(unittest.TestCase):
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

    def test_package_notices_carry_source_package_and_version(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rows = [{'path': 'usr/lib/libgtk-3.so.0', 'type': 'file', 'elf_machine': 62,
                     'host_candidates': [{'package': 'libgtk-3-0t64', 'path': '/usr/lib/libgtk-3.so.0'}]}]
            packages = {'libgtk-3-0t64': {'version': '3.24.49-1',
                                          'source_package': 'gtk+3.0',
                                          'source_version': '3.24.49-1'}}
            entries = inventory.package_notices(root, rows, packages)
            self.assertEqual(len(entries), 1)
            self.assertEqual(entries[0]['package'], 'libgtk-3-0t64')
            self.assertEqual(entries[0]['version'], '3.24.49-1')
            self.assertEqual(entries[0]['source_package'], 'gtk+3.0')
            self.assertEqual(entries[0]['source_version'], '3.24.49-1')

    def test_package_notices_tolerate_an_unknown_package(self):
        with tempfile.TemporaryDirectory() as directory:
            rows = [{'path': 'usr/lib/libmystery.so.1', 'type': 'file', 'elf_machine': 62,
                     'host_candidates': [{'package': 'ghost', 'path': '/usr/lib/libmystery.so.1'}]}]
            entries = inventory.package_notices(Path(directory), rows, {})
            self.assertEqual(entries[0]['version'], '')
            self.assertEqual(entries[0]['source_package'], '')
    def test_write_notices_emits_notice_and_both_lgpl_texts(self):
        from unittest import mock
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            (root / 'usr/lib').mkdir(parents=True)
            repo = Path(directory) / 'repo'
            texts = repo / 'scripts/release_licenses/native'
            texts.mkdir(parents=True)
            (texts / 'HOST-LGPL-2.txt').write_text('lgpl2')
            (texts / 'HOST-LGPL-2.1.txt').write_text('lgpl21')
            with mock.patch('appimage_policy.resolve_libraries', return_value=[
                    {'path': 'usr/lib/libgtk-3.so.0', 'package': 'libgtk-3-0t64',
                     'version': '3.24.49-1', 'source_package': 'gtk+3.0',
                     'source_version': '3.24.49-1', 'labels': ['LGPL-2.1']}]):
                written = inventory.write_notices(root, repo)
            notice = root / 'usr/lib/FontCarver/licenses/appimage/BUNDLED-LIBRARIES.txt'
            self.assertTrue(notice.is_file())
            self.assertIn('usr/lib/libgtk-3.so.0', notice.read_text())
            self.assertEqual((root / 'usr/lib/FontCarver/licenses/appimage/HOST-LGPL-2.txt'
                              ).read_text(), 'lgpl2')
            self.assertEqual(len(written), 3)


if __name__ == '__main__':
    unittest.main()
