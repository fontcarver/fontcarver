"""Staging checks for the installers a tag publishes to one GitHub Release."""

import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import publish_release as publish


REVISION = 'f' * 40
PAYLOADS = {
    'linux-x86_64': [('deb', 'deb/FontCarver_1.2.3_amd64.deb'),
                     ('rpm', 'rpm/FontCarver-1.2.3-1.x86_64.rpm'),
                     ('appimage', 'appimage/FontCarver_1.2.3_amd64.AppImage')],
    'macos-arm64': [('dmg', 'dmg/FontCarver_1.2.3_aarch64.dmg')],
    'windows-x86_64': [('nsis', 'nsis/FontCarver_1.2.3_x64-setup.exe'),
                       ('portable', 'portable/FontCarver_1.2.3_x64-portable.zip')],
}


def write_download(root: Path, labels=None, revision=REVISION, **overrides) -> Path:
    """Build a download tree shaped like `actions/download-artifact` leaves one."""
    for label in labels or PAYLOADS:
        bundle = root / f'FontCarver-{label}-{revision}' / 'target/release/bundle'
        rows = []
        for kind, relative in PAYLOADS[label]:
            artifact = bundle / relative
            artifact.parent.mkdir(parents=True, exist_ok=True)
            artifact.write_bytes(f'{label}:{kind} payload'.encode())
            rows.append({'path': relative, 'type': kind,
                         'bytes': artifact.stat().st_size,
                         'sha256': hashlib.sha256(artifact.read_bytes()).hexdigest()})
        manifest = {'schema_version': 1, 'ok': True,
                    'expected': [kind for kind, _ in PAYLOADS[label]],
                    'artifacts': rows, 'source': {'revision': revision},
                    'signing_verified': False, 'publication_performed': False}
        manifest.update(overrides.get(label, {}))
        (bundle / 'release-manifest.json').write_text(json.dumps(manifest), encoding='utf-8')
    return root


class TagTests(unittest.TestCase):
    def test_a_tag_carries_the_version_without_its_prefix(self):
        self.assertEqual(publish.tag_version('v1.2.3'), '1.2.3')
        self.assertEqual(publish.tag_version('v0.1.0-alpha.1'), '0.1.0-alpha.1')

    def test_a_tag_that_is_not_a_version_is_refused(self):
        for tag in ('1.2.3', 'v1.2', 'release-1.2.3', 'v1.2.3+build', 'v'):
            with self.assertRaises(ValueError):
                publish.tag_version(tag)

    def test_a_prerelease_version_is_recognised_by_its_identifier(self):
        self.assertTrue(publish.is_prerelease('0.1.0-alpha.1'))
        self.assertFalse(publish.is_prerelease('0.1.0'))


class StageTests(unittest.TestCase):
    def setUp(self):
        self.directory = Path(tempfile.mkdtemp(prefix='publish-release-'))
        self.addCleanup(lambda: __import__('shutil').rmtree(self.directory))
        self.source = write_download(self.directory / 'download')
        self.destination = self.directory / 'assets'

    def stage(self, **kwargs):
        arguments = {'source': self.source, 'destination': self.destination,
                     'version': '1.2.3', 'revision': REVISION}
        return publish.stage(**{**arguments, **kwargs})

    def test_every_platform_installer_is_staged_under_one_flat_directory(self):
        report = self.stage()
        staged = sorted(path.name for path in self.destination.iterdir())
        self.assertEqual(staged, [
            'FontCarver-1.2.3-1.x86_64.rpm', 'FontCarver_1.2.3_aarch64.dmg',
            'FontCarver_1.2.3_amd64.AppImage', 'FontCarver_1.2.3_amd64.deb',
            'FontCarver_1.2.3_x64-portable.zip', 'FontCarver_1.2.3_x64-setup.exe',
            'SHA256SUMS.txt', 'release-manifest-linux-x86_64.json',
            'release-manifest-macos-arm64.json', 'release-manifest-windows-x86_64.json'])
        self.assertEqual(len(report['assets']), len(staged))

    def test_a_staged_installer_keeps_the_bytes_the_build_verified(self):
        self.stage()
        original = (self.source / f'FontCarver-macos-arm64-{REVISION}'
                    / 'target/release/bundle/dmg/FontCarver_1.2.3_aarch64.dmg')
        staged = self.destination / 'FontCarver_1.2.3_aarch64.dmg'
        self.assertEqual(staged.read_bytes(), original.read_bytes())

    def test_each_platform_manifest_is_published_unchanged_under_its_label(self):
        self.stage()
        for label in PAYLOADS:
            published = json.loads((self.destination / f'release-manifest-{label}.json')
                                   .read_text(encoding='utf-8'))
            built = json.loads((self.source / f'FontCarver-{label}-{REVISION}'
                                / 'target/release/bundle/release-manifest.json')
                               .read_text(encoding='utf-8'))
            self.assertEqual(published, built)

    def test_the_checksum_file_covers_every_asset_but_itself(self):
        self.stage()
        lines = (self.destination / 'SHA256SUMS.txt').read_text(encoding='utf-8').splitlines()
        named = {line.split('  ', 1)[1] for line in lines}
        self.assertEqual(named, {path.name for path in self.destination.iterdir()}
                         - {'SHA256SUMS.txt'})
        for line in lines:
            digest, name = line.split('  ', 1)
            self.assertEqual(digest,
                             hashlib.sha256((self.destination / name).read_bytes()).hexdigest())

    def test_a_missing_platform_stops_the_release(self):
        source = write_download(self.directory / 'partial', labels=['linux-x86_64'])
        with self.assertRaisesRegex(publish.ReleaseError, 'macos-arm64'):
            self.stage(source=source)

    def test_a_platform_that_failed_its_own_inspection_stops_the_release(self):
        source = write_download(self.directory / 'failed',
                                **{'macos-arm64': {'ok': False, 'error': 'dmg payload missing'}})
        with self.assertRaisesRegex(publish.ReleaseError, 'dmg payload missing'):
            self.stage(source=source)

    def test_a_platform_built_from_another_revision_stops_the_release(self):
        source = write_download(self.directory / 'stale')
        manifest = (source / f'FontCarver-macos-arm64-{REVISION}'
                    / 'target/release/bundle/release-manifest.json')
        report = json.loads(manifest.read_text(encoding='utf-8'))
        report['source']['revision'] = 'a' * 40
        manifest.write_text(json.dumps(report), encoding='utf-8')
        with self.assertRaisesRegex(publish.ReleaseError, 'revision'):
            self.stage(source=source)

    def test_an_installer_whose_bytes_changed_after_inspection_stops_the_release(self):
        source = write_download(self.directory / 'tampered')
        artifact = (source / f'FontCarver-windows-x86_64-{REVISION}'
                    / 'target/release/bundle/nsis/FontCarver_1.2.3_x64-setup.exe')
        artifact.write_bytes(artifact.read_bytes() + b'appended')
        with self.assertRaisesRegex(publish.ReleaseError, 'sha256|digest'):
            self.stage(source=source)

    def test_a_platform_missing_one_of_its_formats_stops_the_release(self):
        source = write_download(self.directory / 'thin')
        manifest = (source / f'FontCarver-linux-x86_64-{REVISION}'
                    / 'target/release/bundle/release-manifest.json')
        report = json.loads(manifest.read_text(encoding='utf-8'))
        report['artifacts'] = [row for row in report['artifacts'] if row['type'] != 'rpm']
        report['expected'] = ['deb', 'appimage']
        manifest.write_text(json.dumps(report), encoding='utf-8')
        with self.assertRaisesRegex(publish.ReleaseError, 'rpm'):
            self.stage(source=source)

    def test_an_installer_that_does_not_carry_the_tag_version_stops_the_release(self):
        with self.assertRaisesRegex(publish.ReleaseError, '1.2.4'):
            self.stage(version='1.2.4')

    def test_the_report_names_the_installer_published_for_each_platform(self):
        report = self.stage()
        self.assertEqual(report['platforms']['windows-x86_64'],
                         ['FontCarver_1.2.3_x64-setup.exe',
                          'FontCarver_1.2.3_x64-portable.zip'])
        self.assertEqual(report['revision'], REVISION)
        self.assertEqual(report['version'], '1.2.3')


class NotesTests(unittest.TestCase):
    def setUp(self):
        self.directory = Path(tempfile.mkdtemp(prefix='publish-notes-'))
        self.addCleanup(lambda: __import__('shutil').rmtree(self.directory))
        source = write_download(self.directory / 'download')
        self.report = publish.stage(source=source, destination=self.directory / 'assets',
                                    version='1.2.3', revision=REVISION)
        self.text = publish.notes(self.report, run_url='https://example.invalid/run/1',
                                  repo_url='https://example.invalid/owner/repo', tag='v1.2.3')

    def test_the_notes_name_every_published_installer(self):
        for asset in self.report['assets']:
            if asset['name'] != 'SHA256SUMS.txt':
                self.assertIn(asset['name'], self.text)

    def test_the_notes_say_the_builds_are_unsigned(self):
        self.assertRegex(self.text, r'(?i)not signed|unsigned')
        self.assertRegex(self.text, r'(?i)notariz')
        self.assertRegex(self.text, r'(?i)smartscreen')

    def test_the_notes_warn_that_a_later_final_version_sorts_below_a_prerelease(self):
        prerelease = publish.notes({**self.report, 'version': '1.2.3-alpha.1'},
                                   run_url='https://example.invalid/run/1')
        self.assertRegex(prerelease, r'(?i)downgrade')
        self.assertNotRegex(self.text, r'(?i)downgrade')

    def test_the_notes_point_at_the_run_and_the_revision_that_built_them(self):
        self.assertIn('https://example.invalid/run/1', self.text)
        self.assertIn(REVISION, self.text)

    def test_a_document_link_is_absolute_because_a_release_body_has_no_repository_root(self):
        self.assertIn('https://example.invalid/owner/repo/blob/v1.2.3/docs/install.md', self.text)
        self.assertNotIn('](docs/', self.text)

    def test_the_notes_say_the_native_acceptance_checklist_is_not_done(self):
        self.assertRegex(self.text, r'(?i)not been (completed|run)|remains outstanding')
        self.assertIn('docs/releasing.md', self.text)


if __name__ == '__main__':
    unittest.main()
