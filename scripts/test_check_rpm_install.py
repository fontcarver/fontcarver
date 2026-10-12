"""Install/remove verification for an rpm candidate inside a throwaway container."""

import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))

import check_rpm_install as install


class ContainerCommandTests(unittest.TestCase):
    def test_the_rpm_is_mounted_read_only_and_never_installed_on_the_host(self):
        argv = install.container_command('podman', 'fedora:41', Path('/out/F.rpm'), 'script')
        self.assertEqual(argv[:2], ['podman', 'run'])
        self.assertIn('--rm', argv)
        self.assertIn('/out/F.rpm:/candidate.rpm:ro', argv)
        self.assertNotIn('--privileged', argv)
        # No display, so the smoke probe exercises the refusal path as the deb
        # check does rather than hanging on a window system.
        self.assertNotIn('DISPLAY', ' '.join(argv))
        self.assertEqual(argv[-3:], ['fedora:41', 'bash', '-eu'])

    def test_the_script_installs_verifies_and_removes_by_package_name(self):
        script = install.container_script('0.1.0-alpha.1')
        for fragment in ('dnf -y install /candidate.rpm',
                         'rpm -q --queryformat',
                         '/usr/bin/fontcarver-desktop',
                         'rpm -V',
                         'dnf -y remove',
                         '0.1.0-alpha.1'):
            self.assertIn(fragment, script)
        # Removal has to be proven, not assumed: the binary must be gone after.
        self.assertLess(script.index('dnf -y remove'),
                        script.rindex('test ! -e /usr/bin/fontcarver-desktop'))

    def test_a_missing_container_runtime_is_named_not_a_traceback(self):
        with patch.object(install.shutil, 'which', return_value=None):
            with self.assertRaisesRegex(RuntimeError, 'podman or docker'):
                install.runtime()


class ReportTests(unittest.TestCase):
    def test_the_report_records_what_was_and_was_not_verified(self):
        with tempfile.TemporaryDirectory() as work:
            artifact = Path(work) / 'FontCarver-0.1.0-1.x86_64.rpm'
            artifact.write_bytes(b'\xed\xab\xee\xdb')
            output = Path(work) / 'report.json'
            with patch.object(install, 'runtime', return_value='docker'), \
                 patch.object(install, 'command', return_value='ok\n') as run:
                self.assertEqual(install.main(['--artifact', str(artifact),
                                               '--version', '0.1.0-alpha.1',
                                               '--output', str(output)]), 0)
            report = json.loads(output.read_text(encoding='utf-8'))
            self.assertEqual(report['version'], '0.1.0-alpha.1')
            self.assertEqual(report['image'], install.DEFAULT_IMAGE)
            self.assertIn('install', ' '.join(report['checks']).lower())
            self.assertTrue(report['limits'])
            self.assertEqual(run.call_count, 1)

    def test_a_failing_container_run_writes_no_success_report(self):
        with tempfile.TemporaryDirectory() as work:
            artifact = Path(work) / 'x.rpm'
            artifact.write_bytes(b'\xed\xab\xee\xdb')
            output = Path(work) / 'report.json'
            with patch.object(install, 'runtime', return_value='docker'), \
                 patch.object(install, 'command', side_effect=RuntimeError('dnf exited 1')):
                with self.assertRaisesRegex(RuntimeError, 'dnf exited 1'):
                    install.main(['--artifact', str(artifact), '--version', '1.0.0',
                                  '--output', str(output)])
            self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()
