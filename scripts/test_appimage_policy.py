"""Release gate tests for a bundled-library AppImage payload."""
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import appimage_policy as policy


def _payload(root: Path, libraries: dict[str, str]) -> None:
    """Create a payload with one application ELF plus the named shared objects."""
    binary = root / 'usr/bin/fontcarver-desktop'
    binary.parent.mkdir(parents=True, exist_ok=True)
    binary.write_bytes(b'\x7fELF\x02\x01\x01' + b'\x00' * 13)
    for name in libraries:
        target = root / 'usr/lib' / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(b'\x7fELF\x02\x01\x01' + b'\x00' * 13)


def _notices(root: Path, listed: str, texts: bool = True) -> None:
    """Create the notice directory the gate requires, with optional LGPL texts."""
    target = root / policy.NOTICE_DIR
    target.mkdir(parents=True, exist_ok=True)
    (target / policy.NOTICE_NAME).write_text(listed)
    if texts:
        for name in policy.REQUIRED_TEXTS:
            (target / name).write_text('license text')


class GateTests(unittest.TestCase):
    def test_missing_arguments_are_refused_without_touching_the_disk(self):
        with self.assertRaisesRegex(ValueError, 'payload and artifact'):
            policy.require_releasable_appimage()

    def test_lgpl_payload_passes_and_reports_every_library(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'libgtk-3.so.0': 'libgtk-3-0t64'})
            _notices(root, 'usr/lib/libgtk-3.so.0\n')
            with mock.patch.object(policy, 'resolve_libraries', return_value=[
                    {'path': 'usr/lib/libgtk-3.so.0', 'package': 'libgtk-3-0t64',
                     'version': '3.24.49-1', 'source_package': 'gtk+3.0',
                     'source_version': '3.24.49-1', 'labels': ['LGPL-2.1']}]):
                report = policy.require_releasable_appimage(root, root)
            self.assertEqual(report['bundled_shared_libraries'], 1)
            self.assertEqual(report['forbidden'], [])

    def test_missing_lgpl_text_fails_even_when_the_notice_is_complete(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'libgtk-3.so.0': 'libgtk-3-0t64'})
            _notices(root, 'usr/lib/libgtk-3.so.0\n', texts=False)
            with mock.patch.object(policy, 'resolve_libraries', return_value=[
                    {'path': 'usr/lib/libgtk-3.so.0', 'package': 'libgtk-3-0t64',
                     'version': '3.24.49-1', 'source_package': 'gtk+3.0',
                     'source_version': '3.24.49-1', 'labels': ['LGPL-2.1']}]):
                with self.assertRaisesRegex(ValueError, 'HOST-LGPL-2.txt'):
                    policy.require_releasable_appimage(root, root)

    def test_gpl_library_without_a_ruling_fails_and_is_named(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'libthing.so.1': 'thing'})
            _notices(root, 'usr/lib/libthing.so.1\n')
            with mock.patch.object(policy, 'resolve_libraries', return_value=[
                    {'path': 'usr/lib/libthing.so.1', 'package': 'thing',
                     'version': '1', 'source_package': 'thing', 'source_version': '1',
                     'labels': ['GPL-3']}]):
                with self.assertRaisesRegex(ValueError, 'libthing.so.1'):
                    policy.require_releasable_appimage(root, root)

    def test_library_with_no_owning_package_fails_and_is_named(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'liborphan.so.2': ''})
            _notices(root, 'usr/lib/liborphan.so.2\n')
            with mock.patch.object(policy, 'resolve_libraries', return_value=[
                    {'path': 'usr/lib/liborphan.so.2', 'package': '', 'version': '',
                     'source_package': '', 'source_version': '', 'labels': []}]):
                with self.assertRaisesRegex(ValueError, 'liborphan.so.2'):
                    policy.require_releasable_appimage(root, root)

    def test_a_file_with_no_owning_package_can_be_ruled_on_by_its_path(self):
        # The AppImage launcher the bundler installs belongs to no Debian
        # package at all, so no package ruling can ever reach it.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'liborphan.so.2': ''})
            _notices(root, 'AppRun.wrapped\n')
            rows = [{'path': 'AppRun.wrapped', 'package': '', 'version': '',
                     'source_package': '', 'source_version': '', 'labels': []}]
            with mock.patch.object(policy, 'resolve_libraries', return_value=rows):
                with mock.patch.dict(policy.PATH_RULINGS, {}, clear=True):
                    with self.assertRaisesRegex(ValueError, 'AppRun.wrapped'):
                        policy.require_releasable_appimage(root, root)
                report = policy.require_releasable_appimage(root, root)
            self.assertEqual(report['bundled_shared_libraries'], 1)

    def test_notice_missing_a_library_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'libgtk-3.so.0': 'libgtk-3-0t64'})
            _notices(root, 'nothing here\n')
            with mock.patch.object(policy, 'resolve_libraries', return_value=[
                    {'path': 'usr/lib/libgtk-3.so.0', 'package': 'libgtk-3-0t64',
                     'version': '3.24.49-1', 'source_package': 'gtk+3.0',
                     'source_version': '3.24.49-1', 'labels': ['LGPL-2.1']}]):
                with self.assertRaisesRegex(ValueError, 'notice does not account'):
                    policy.require_releasable_appimage(root, root)

    def test_absent_notice_directory_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'libgtk-3.so.0': 'libgtk-3-0t64'})
            with mock.patch.object(policy, 'resolve_libraries', return_value=[
                    {'path': 'usr/lib/libgtk-3.so.0', 'package': 'libgtk-3-0t64',
                     'version': '3.24.49-1', 'source_package': 'gtk+3.0',
                     'source_version': '3.24.49-1', 'labels': ['LGPL-2.1']}]):
                with self.assertRaisesRegex(ValueError, 'BUNDLED-LIBRARIES.txt'):
                    policy.require_releasable_appimage(root, root)


class NoticeCoverageTests(unittest.TestCase):
    def test_a_path_that_is_a_prefix_of_another_is_not_counted_as_documented(self):
        # Payloads routinely carry both libfoo.so.1 and libfoo.so.1.2 as real
        # files, so a substring test would pass the shorter one undocumented.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'libfoo.so.1': 'foo', 'libfoo.so.1.2': 'foo'})
            _notices(root, 'usr/lib/libfoo.so.1.2\n')
            with mock.patch.object(policy, 'resolve_libraries', return_value=[
                    {'path': 'usr/lib/libfoo.so.1', 'package': 'foo', 'version': '1',
                     'source_package': 'foo', 'source_version': '1', 'labels': ['MIT']},
                    {'path': 'usr/lib/libfoo.so.1.2', 'package': 'foo', 'version': '1',
                     'source_package': 'foo', 'source_version': '1', 'labels': ['MIT']}]):
                with self.assertRaisesRegex(ValueError, r'libfoo\.so\.1$'):
                    policy.require_releasable_appimage(root, root)


class CandidateChoiceTests(unittest.TestCase):
    def test_the_most_restrictive_candidate_decides_not_the_first(self):
        # host_inventory orders candidates alphabetically by package name, so a
        # permissive 'afoo' must not shadow a GPL 'zfoo-gpl' owning the same name.
        notices = {'afoo': {'package': 'afoo', 'version': '1', 'source_package': 'afoo',
                            'source_version': '1', 'labels': ['MIT']},
                   'zfoo-gpl': {'package': 'zfoo-gpl', 'version': '1',
                                'source_package': 'zfoo-gpl', 'source_version': '1',
                                'labels': ['GPL-3']}}
        chosen = policy.choose_candidate(
            [{'package': 'afoo'}, {'package': 'zfoo-gpl'}], notices)
        self.assertEqual(chosen['package'], 'zfoo-gpl')

    def test_an_exact_content_match_is_preferred_over_a_name_only_match(self):
        notices = {'afoo': {'package': 'afoo', 'version': '1', 'source_package': 'afoo',
                            'source_version': '1', 'labels': ['MIT']},
                   'bfoo': {'package': 'bfoo', 'version': '1', 'source_package': 'bfoo',
                            'source_version': '1', 'labels': ['MIT']}}
        chosen = policy.choose_candidate(
            [{'package': 'afoo', 'content_identical': False},
             {'package': 'bfoo', 'content_identical': True}], notices)
        self.assertEqual(chosen['package'], 'bfoo')

    def test_no_candidates_yields_nothing(self):
        self.assertIsNone(policy.choose_candidate([], {}))


class GatedRowTests(unittest.TestCase):
    def test_a_bundled_elf_executable_is_gated_even_without_a_so_name(self):
        rows = [{'path': 'usr/bin/WebKitWebProcess', 'elf_machine': 62},
                {'path': 'usr/lib/libgtk-3.so.0', 'elf_machine': 62}]
        gated = [row['path'] for row in policy.gated_rows(rows)]
        self.assertIn('usr/bin/WebKitWebProcess', gated)
        self.assertIn('usr/lib/libgtk-3.so.0', gated)

    def test_the_application_binary_and_non_elf_files_are_not_gated(self):
        rows = [{'path': policy.APPLICATION_ELF, 'elf_machine': 62},
                {'path': 'AppRun'},
                {'path': 'usr/share/applications/app.desktop'}]
        self.assertEqual(policy.gated_rows(rows), [])


if __name__ == '__main__':
    unittest.main()
