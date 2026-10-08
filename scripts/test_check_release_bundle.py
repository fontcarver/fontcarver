"""Exercise package discovery and payload checks without building the desktop app."""
import json
import os
import plistlib
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import check_release_bundle as verifier
from check_release_licenses import HOST_LICENSE_HASHES


class ReleaseBundleTests(unittest.TestCase):
    def setUp(self):
        self.scratch = verifier.ROOT / '.scratch' / 'p5-release-verify-tests'
        self.scratch.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=self.scratch)
        self.root = Path(self.temporary.name)

    def tearDown(self):
        self.temporary.cleanup()
        try:
            self.scratch.rmdir()
        except OSError:
            pass

    def payload(self):
        binary = self.root / 'usr/bin/fontcarver-desktop'
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b'\x7fELF' + b'\x00' * 64 + b'__TAURI_BUNDLE_TYPE_VAR_DEB')
        binary.chmod(0o755)
        resources = self.root / 'usr/lib/fontcarver'
        resources.mkdir(parents=True)
        for name in verifier.RESOURCES:
            (resources / name).write_text('FontForge is a behavioral reference.\n')
        notice = resources / 'licenses' / 'dependency.txt'
        notice.parent.mkdir()
        notice.write_text('Permission to redistribute.\n')
        for name in ('scripts/release_licenses/native/HOST-LIBRARIES.txt', *HOST_LICENSE_HASHES):
            target = resources / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(verifier.ROOT / name, target)
        self.dependencies = resources / 'DEPENDENCIES.json'
        self.dependencies.write_text(json.dumps({
            'schema_version': 1,
            'packages': [{'ecosystem': 'cargo', 'name': 'example', 'version': '1.0',
                          'license': 'MIT', 'notices': [{'path': 'licenses/dependency.txt',
                                                      'sha256': verifier.sha256(notice)}]}],
        }))
        desktop = self.root / 'usr/share/applications/fontcarver-desktop.desktop'
        desktop.parent.mkdir(parents=True)
        desktop.write_text('[Desktop Entry]\nType=Application\nName=FontCarver\n'
                           'Exec=fontcarver-desktop\nIcon=fontcarver-desktop\n')
        icon = self.root / 'usr/share/icons/hicolor/32x32/apps/fontcarver-desktop.png'
        icon.parent.mkdir(parents=True)
        icon.write_bytes(b'\x89PNG\r\n\x1a\nicon')
        # Debian policy 12.5 makes this mandatory, and the directory repeats the
        # package name the real `Package` field carries.
        self.copyright = self.root / 'usr/share/doc/fontcarver/copyright'
        self.copyright.parent.mkdir(parents=True)
        self.copyright.write_text('Files: *\nLicense: MIT or Apache-2.0\n')
        return resources

    def test_complete_payload_and_legal_fontforge_mentions_pass(self):
        self.payload()
        report = verifier.inspect_payload(self.root, 'deb')
        self.assertTrue(report['executable']['valid_header'])
        self.assertEqual(report['notices']['packages'], 1)
        self.assertEqual(len(report['resources']), len(verifier.RESOURCES))

    def test_linux_host_library_notice_and_original_license_texts_are_required(self):
        resources = self.payload()
        notice = resources / 'scripts/release_licenses/native/HOST-LIBRARIES.txt'
        notice.unlink()
        with self.assertRaisesRegex(verifier.VerificationError, 'host-library notice'):
            verifier.inspect_payload(self.root, 'deb')
        notice.write_text('Host library notice')
        license_file = resources / next(iter(HOST_LICENSE_HASHES))
        license_file.write_text('Changed license terms')
        with self.assertRaisesRegex(verifier.VerificationError, 'host-library license checksum'):
            verifier.inspect_payload(self.root, 'deb')

    def test_debian_bundle_marker_must_be_unique_and_identify_deb(self):
        self.payload()
        binary = self.root / 'usr/bin/fontcarver-desktop'
        prefix = b'__TAURI_BUNDLE_TYPE_VAR_'
        for markers in (b'', prefix + b'UNK', prefix + b'APP',
                        prefix + b'DEB' + prefix + b'DEB', prefix + b'DEB' + prefix + b'APP'):
            with self.subTest(markers=markers):
                binary.write_bytes(b'\x7fELF' + b'\x00' * 64 + markers)
                with self.assertRaisesRegex(verifier.VerificationError, 'Tauri bundle marker'):
                    verifier.inspect_payload(self.root, 'deb')
        binary.write_bytes(b'\x7fELF' + b'\x00' * 64 + prefix + b'DEB')
        report = verifier.inspect_payload(self.root, 'deb')
        self.assertEqual(report['tauri_bundle_marker'], {'type': 'DEB', 'offset': 68})

    def test_missing_and_empty_resources_are_rejected(self):
        resources = self.payload()
        license_file = resources / 'LICENSE-MIT'
        license_file.write_bytes(b'')
        with self.assertRaisesRegex(verifier.VerificationError, 'LICENSE-MIT'):
            verifier.inspect_payload(self.root, 'deb')
        license_file.unlink()
        with self.assertRaisesRegex(verifier.VerificationError, 'LICENSE-MIT'):
            verifier.inspect_payload(self.root, 'deb')

    def test_plain_text_cannot_satisfy_application_executable(self):
        self.payload()
        (self.root / 'usr/bin/fontcarver-desktop').write_text('not a binary')
        with self.assertRaisesRegex(verifier.VerificationError, 'executable'):
            verifier.inspect_payload(self.root, 'deb')

    def test_fontforge_binary_library_and_source_archive_are_rejected(self):
        self.payload()
        for name in ('FontForge.exe', 'libfontforge.so.4', 'libfontforge-4.dll',
                     'fontforge-20230101.tar.xz', 'fontforge-source.zip', 'fontforge.py'):
            with self.subTest(name=name):
                forbidden = self.root / name
                forbidden.write_bytes(b'not shipped')
                with self.assertRaisesRegex(verifier.VerificationError, 'FontForge'):
                    verifier.inspect_payload(self.root, 'deb')
                forbidden.unlink()

    def test_modified_dependency_license_is_rejected(self):
        resources = self.payload()
        (resources / 'licenses/dependency.txt').write_text('Changed after notice generation')
        with self.assertRaisesRegex(verifier.VerificationError, 'checksum'):
            verifier.inspect_payload(self.root, 'deb')

    def test_dependency_notice_cannot_escape_resource_directory(self):
        self.payload()
        data = json.loads(self.dependencies.read_text())
        data['packages'][0]['notices'][0]['path'] = '../LICENSE-MIT'
        self.dependencies.write_text(json.dumps(data))
        with self.assertRaisesRegex(verifier.VerificationError, 'notice path'):
            verifier.inspect_payload(self.root, 'deb')

    @unittest.skipIf(os.name == 'nt', 'Creating symlinks requires Windows privileges')
    def test_external_resource_symlink_is_rejected(self):
        resources = self.payload()
        resource = resources / 'INSTALL.md'
        resource.unlink()
        resource.symlink_to(verifier.ROOT / 'CONVENTIONS.md')
        with self.assertRaisesRegex(verifier.VerificationError, 'outside'):
            verifier.inspect_payload(self.root, 'deb')

    def test_appimage_requires_launcher_and_desktop_metadata(self):
        self.payload()
        with self.assertRaisesRegex(verifier.VerificationError, 'AppRun'):
            verifier.inspect_payload(self.root, 'appimage')

    @unittest.skipIf(os.name == 'nt', 'POSIX executable permissions')
    def test_appimage_launcher_must_be_executable(self):
        self.payload()
        launcher = self.root / 'AppRun'
        launcher.write_text('#!/bin/sh\nexec usr/bin/fontcarver-desktop\n')
        launcher.chmod(0o644)
        with self.assertRaisesRegex(verifier.VerificationError, 'AppRun'):
            verifier.inspect_payload(self.root, 'appimage')
        launcher.chmod(0o755)
        self.assertTrue(verifier.inspect_payload(self.root, 'appimage')['executable']['valid_header'])

    def test_linux_executable_must_be_installed_on_path(self):
        self.payload()
        (self.root / 'usr/bin/fontcarver-desktop').rename(self.root / 'fontcarver-desktop')
        with self.assertRaisesRegex(verifier.VerificationError, 'executable'):
            verifier.inspect_payload(self.root, 'deb')

    def test_instructions_can_live_above_dependency_notices(self):
        resources = self.payload()
        (resources / 'INSTALL.md').rename(resources.parent / 'INSTALL.md')
        dependencies = resources.parent / 'licenses/dependencies'
        dependencies.parent.mkdir()
        resources.rename(dependencies)
        report = verifier.inspect_payload(self.root, 'deb')
        self.assertEqual(report['notices']['packages'], 1)
        self.assertIn('usr/lib/INSTALL.md', [item['path'] for item in report['resources']])

    def test_expected_artifacts_are_discovered_case_insensitively(self):
        (self.root / 'deb').mkdir()
        deb = self.root / 'deb/FontCarver.deb'
        appimage = self.root / 'FontCarver.AppImage'
        deb.write_bytes(b'deb')
        appimage.write_bytes(b'appimage')
        discovered = verifier.discover(self.root, ['deb', 'appimage'])
        self.assertEqual(discovered, [('deb', deb), ('appimage', appimage)])

    def test_missing_expected_type_and_empty_installer_are_rejected(self):
        (self.root / 'FontCarver.deb').touch()
        with self.assertRaisesRegex(verifier.VerificationError, 'empty'):
            verifier.discover(self.root, ['deb'])
        with self.assertRaisesRegex(verifier.VerificationError, 'msi'):
            verifier.discover(self.root, ['msi'])

    def test_native_platform_mismatch_is_not_reported_as_success(self):
        with patch.object(verifier.sys, 'platform', 'linux'):
            with self.assertRaisesRegex(verifier.VerificationError, 'darwin'):
                with verifier.extracted(self.root / 'test.dmg', 'dmg', self.root):
                    self.fail('Unsupported platform yielded an extracted payload')

    def test_dmg_is_detached_when_payload_validation_fails(self):
        calls = []

        def command(arguments, cwd=None):
            calls.append(arguments)
            return plistlib.dumps({'system-entities': []})

        with patch.object(verifier.sys, 'platform', 'darwin'), patch.object(verifier, 'run', command):
            with self.assertRaisesRegex(verifier.VerificationError, 'payload invalid'):
                with verifier.extracted(self.root / 'test.dmg', 'dmg', self.root):
                    raise verifier.VerificationError('payload invalid')
        self.assertIn('-readonly', calls[0])
        self.assertEqual(calls[-1][:2], ['hdiutil', 'detach'])

    def test_failed_extraction_removes_temporary_directory(self):
        (self.root / 'test.deb').write_bytes(b'not a deb')
        with patch.object(verifier, 'ROOT', self.root), patch.object(verifier.sys, 'platform', 'linux'):
            with patch.object(verifier, 'run', side_effect=verifier.VerificationError('bad archive')):
                with self.assertRaisesRegex(verifier.VerificationError, 'bad archive'):
                    verifier.verify(self.root, ['deb'])
            self.assertFalse((self.root / '.scratch/p5-release-verify').exists())

    def test_debian_package_without_a_copyright_document_is_rejected(self):
        self.payload()
        self.copyright.unlink()
        with self.assertRaisesRegex(verifier.VerificationError, 'copyright'):
            verifier.inspect_payload(self.root, 'deb')
        self.copyright.write_bytes(b'')
        with self.assertRaisesRegex(verifier.VerificationError, 'empty Debian copyright'):
            verifier.inspect_payload(self.root, 'deb')

    def test_copyright_document_must_be_in_the_installed_package_directory(self):
        self.payload()
        report = verifier.inspect_payload(self.root, 'deb', {'Package': 'fontcarver'})
        self.assertEqual(report['debian_copyright']['path'],
                         'usr/share/doc/fontcarver/copyright')
        # A renamed product moves the installed directory; a document left at the
        # old name is unreachable and must not pass.
        with self.assertRaisesRegex(verifier.VerificationError, 'font-carver'):
            verifier.inspect_payload(self.root, 'deb', {'Package': 'font-carver'})

    def test_copyright_document_must_name_both_project_licenses(self):
        self.payload()
        self.copyright.write_text('Files: *\nLicense: MIT\n')
        with self.assertRaisesRegex(verifier.VerificationError, 'Apache-2.0'):
            verifier.inspect_payload(self.root, 'deb')
        self.copyright.write_text('Files: *\nLicense: Apache-2.0\n')
        with self.assertRaisesRegex(verifier.VerificationError, 'MIT'):
            verifier.inspect_payload(self.root, 'deb')

    def test_debian_expat_spelling_satisfies_the_mit_half(self):
        self.payload()
        # A DEP-5 document may use Debian's own short name for the MIT license.
        self.copyright.write_text('Files: *\nLicense: Expat or Apache-2.0\n')
        report = verifier.inspect_payload(self.root, 'deb')
        self.assertEqual(report['debian_copyright']['path'],
                         'usr/share/doc/fontcarver/copyright')

    def test_appimage_does_not_require_the_debian_copyright_document(self):
        self.payload()
        self.copyright.unlink()
        launcher = self.root / 'AppRun'
        launcher.write_text('#!/bin/sh\nexec usr/bin/fontcarver-desktop\n')
        launcher.chmod(0o755)
        report = verifier.inspect_payload(self.root, 'appimage')
        self.assertNotIn('debian_copyright', report)

    def test_real_deb_round_trip_checks_metadata_and_checksum(self):
        # This is the only case that shells out to dpkg-deb, so it is the only
        # one that may be skipped for a missing dpkg-deb. The guard lives in the
        # body rather than in a decorator because a decorator silently transfers
        # to whatever test is inserted after it, which would both skip a case
        # that needs no dpkg-deb and leave this one to error without it.
        if shutil.which('dpkg-deb') is None:
            self.skipTest('dpkg-deb is required for native extraction')
        outer = self.root
        self.root = outer / 'source'
        self.payload()
        control = self.root / 'DEBIAN/control'
        control.parent.mkdir()
        control.write_text('Package: fontcarver\nVersion: 0.1.0\nArchitecture: amd64\n'
                           'Maintainer: FontCarver <test@example.invalid>\n'
                           'Description: Synthetic package extraction test\n')
        artifact = outer / 'FontCarver.deb'
        verifier.run(['dpkg-deb', '--build', str(self.root), str(artifact)])
        report = verifier.verify(outer, ['deb'])
        self.assertEqual(report[0]['metadata']['Package'], 'fontcarver')
        self.assertEqual(report[0]['sha256'], verifier.sha256(artifact))

    def test_rpm_metadata_requires_name_version_and_architecture(self):
        artifact = self.root / 'FontCarver-0.1.0-1.x86_64.rpm'
        artifact.write_bytes(b'\xed\xab\xee\xdb')
        scratch = self.root / 'scratch'
        scratch.mkdir()
        with patch.object(verifier.shutil, 'which', return_value='/usr/bin/tool'):
            with patch.object(verifier, 'run', return_value=b'FontCarver\n'):
                with self.assertRaisesRegex(verifier.VerificationError, 'NAME, VERSION'):
                    with verifier.extracted(artifact, 'rpm', scratch):
                        pass

    def test_rpm_tool_guard_runs_before_the_rpm_query(self):
        # On a host with no rpm at all the message must name the prerequisites, not
        # surface a FileNotFoundError from the query.
        artifact = self.root / 'FontCarver-0.1.0-1.x86_64.rpm'
        artifact.write_bytes(b'\xed\xab\xee\xdb')
        scratch = self.root / 'scratch'
        scratch.mkdir()
        with patch.object(verifier, 'run', side_effect=AssertionError('queried too early')):
            with patch.object(verifier.shutil, 'which', return_value=None):
                with self.assertRaisesRegex(verifier.VerificationError, 'rpm2cpio'):
                    with verifier.extracted(artifact, 'rpm', scratch):
                        pass

    def test_rpm_payload_is_inspected_like_a_deb(self):
        self.payload()
        self.assertEqual(verifier.HOSTS['rpm'], 'linux')
        report = verifier.inspect_payload(self.root, 'rpm')
        self.assertEqual(report['executable']['paths'], ['usr/bin/fontcarver-desktop'])
        self.assertIn('host_library_notices', report)
        self.assertIn('desktop', report['platform_metadata'])

    def test_nsis_replaces_msi_as_the_windows_target(self):
        self.assertEqual(verifier.HOSTS['nsis'], 'win32')
        self.assertNotIn('msi', verifier.HOSTS)

    def test_nsis_artifacts_are_discovered_by_their_installer_name(self):
        self.assertEqual(verifier.SUFFIXES['nsis'], '-setup.exe')
        self.assertEqual(verifier.SUFFIXES['rpm'], '.rpm')
        bundle = self.root / 'bundle'
        (bundle / 'nsis').mkdir(parents=True)
        installer = bundle / 'nsis/FontCarver_0.1.0_x64-setup.exe'
        installer.write_bytes(b'MZ')
        # A stray executable under the bundle tree must not be taken for the
        # installer: discover would otherwise report two candidates and fail a
        # correct build.
        (bundle / 'MicrosoftEdgeWebview2Setup.exe').write_bytes(b'MZ')
        (bundle / 'uninstall.exe').write_bytes(b'MZ')
        self.assertEqual(verifier.discover(bundle, ['nsis']), [('nsis', installer)])

    def test_nsis_extraction_tolerates_a_7z_warning_but_not_a_failure(self):
        from types import SimpleNamespace
        payload = self.root / 'payload'
        payload.mkdir()

        def warn(*args, **kwargs):
            (payload / 'fontcarver-desktop.exe').write_bytes(b'MZ')
            return SimpleNamespace(returncode=1, stderr=b'WARNING: cannot open file')

        with patch.object(verifier.subprocess, 'run', side_effect=warn):
            verifier.extract_nsis(self.root / 'x-setup.exe', payload)
        with patch.object(verifier.subprocess, 'run',
                          return_value=SimpleNamespace(returncode=2, stderr=b'fatal')):
            with self.assertRaisesRegex(verifier.VerificationError, 'nsis installer'):
                verifier.extract_nsis(self.root / 'x-setup.exe', payload)

    def test_rpm_copyright_is_verified_like_the_deb_one(self):
        self.payload()
        report = verifier.inspect_payload(self.root, 'rpm', {'Package': 'fontcarver'})
        self.assertIn('debian_copyright', report)
        self.assertNotIn('tauri_bundle_marker', report)

    def test_nsis_extraction_rejects_an_empty_payload(self):
        # 7z exits 0 on archives it only partly understands; an empty payload must
        # not reach the inspector as a misleading "missing executable".
        artifact = self.root / 'FontCarver_0.1.0_x64-setup.exe'
        artifact.write_bytes(b'MZ')
        scratch = self.root / 'scratch'
        scratch.mkdir()
        from types import SimpleNamespace
        with patch.object(verifier, 'sys') as fake_sys:
            fake_sys.platform = 'win32'
            with patch.object(verifier.subprocess, 'run',
                              return_value=SimpleNamespace(returncode=0, stderr=b'')):
                with self.assertRaisesRegex(verifier.VerificationError, 'extracted no files'):
                    with verifier.extracted(artifact, 'nsis', scratch):
                        pass


    def test_portable_zip_is_a_windows_artifact_matched_by_name(self):
        self.assertEqual(verifier.HOSTS['portable'], 'win32')
        self.assertEqual(verifier.SUFFIXES['portable'], '-portable.zip')
        bundle = self.root / 'bundle'
        (bundle / 'portable').mkdir(parents=True)
        archive = bundle / 'portable/FontCarver_0.1.0_x64-portable.zip'
        archive.write_bytes(b'PK')
        self.assertEqual(verifier.discover(bundle, ['portable']), [('portable', archive)])

    def test_portable_zip_member_escaping_its_folder_is_rejected(self):
        import zipfile
        archive = self.root / 'FontCarver_0.1.0_x64-portable.zip'
        with zipfile.ZipFile(archive, 'w') as zip_file:
            zip_file.writestr('FontCarver-0.1.0/fontcarver-desktop.exe', b'MZ')
            zip_file.writestr('FontCarver-0.1.0/../../escaped.txt', b'x')
        scratch = self.root / 'scratch'
        scratch.mkdir()
        with patch.object(verifier, 'sys') as fake_sys:
            fake_sys.platform = 'win32'
            with self.assertRaisesRegex(verifier.VerificationError, 'escapes'):
                with verifier.extracted(archive, 'portable', scratch):
                    pass

if __name__ == '__main__':
    unittest.main()
