"""Regression checks for rootless APT isolation and failure propagation."""
from pathlib import Path
import os
import io
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import MagicMock, patch

import check_deb_install as install


class DebInstallTests(unittest.TestCase):
    def test_package_paths_are_writable_only_through_owned_overlays(self):
        args = install.sandbox_command(Path('/owned/scratch'), Path('/candidate.deb'), '0.1.0')
        self.assertEqual(args[args.index('--ro-bind') + 1:args.index('--ro-bind') + 3], ['/', '/'])
        binds = [args[index + 1:index + 3] for index, value in enumerate(args) if value == '--bind']
        self.assertEqual(binds, [[f'/owned/scratch/{name}/merged', '/' + name]
                                 for name in ('usr', 'etc', 'var')])
        self.assertIn('--unshare-net', args)
        self.assertIn('--die-with-parent', args)
        for name in ('DISPLAY', 'WAYLAND_DISPLAY', 'DBUS_SESSION_BUS_ADDRESS', 'XDG_RUNTIME_DIR'):
            self.assertIn(['--unsetenv', name], [args[i:i + 2] for i in range(len(args))])
        self.assertIn(['--setenv', 'HOME', '/tmp/home'], [args[i:i + 3] for i in range(len(args))])

    def test_nested_icon_parents_are_owned_before_overlay_copy_up(self):
        stream = io.BytesIO()
        with tarfile.open(fileobj=stream, mode="w") as archive:
            archive.addfile(tarfile.TarInfo("./usr/share/icons/hicolor/128x128/apps/fontcarver.png"))
        self.assertIn(install.PurePosixPath("share/icons/hicolor/128x128/apps"),
                      install.package_directories(stream.getvalue()))
        stream = io.BytesIO()
        with tarfile.open(fileobj=stream, mode="w") as archive:
            archive.addfile(tarfile.TarInfo("./usr/../../outside"))
        with self.assertRaisesRegex(RuntimeError, "unsafe package path"):
            install.package_directories(stream.getvalue())

    def test_apt_must_not_download_or_reuse_host_archive_cache(self):
        args = install.apt_arguments('reinstall', '/tmp/fontcarver.deb')
        self.assertIn('--no-download', args)
        self.assertIn('--reinstall', args)
        self.assertIn('Dir::Cache::archives=/tmp/apt-archives', args)
        self.assertEqual(args[-2:], ['install', '/tmp/fontcarver.deb'])
        self.assertEqual(install.apt_arguments('remove', 'font-carver')[-2:], ['remove', 'font-carver'])

    @unittest.skipIf(os.name == 'nt', 'Unix process-group cleanup')
    def test_command_failure_cannot_be_recorded_as_pass(self):
        result = MagicMock(returncode=100, pid=999999)
        with patch.object(install.subprocess, 'Popen', return_value=result), \
                patch.object(install.os, 'killpg', create=True):
            with self.assertRaisesRegex(RuntimeError, 'exited 100'):
                install.command(['apt-get'])

    @unittest.skipIf(os.name == 'nt', 'Unix process-group cleanup')
    def test_timeout_kills_owned_group_and_is_reported_as_failure(self):
        process = MagicMock(returncode=-9, pid=999999)
        process.wait.side_effect = [subprocess.TimeoutExpired("apt-get", 1), None]
        with patch.object(install.subprocess, "Popen", return_value=process), \
                patch.object(install.os, "killpg") as kill:
            with self.assertRaisesRegex(RuntimeError, "timed out"):
                install.command(["apt-get"], timeout=1)
            kill.assert_called_once_with(999999, install.signal.SIGKILL)

    @unittest.skipIf(os.name == 'nt', 'POSIX directory permission semantics')
    def test_cleanup_removes_private_mode_zero_overlay_workdir(self):
        root = install.ROOT / '.scratch/p5f-w3-packaging'
        root.mkdir(parents=True, exist_ok=True)
        path = Path(tempfile.mkdtemp(dir=root))
        (path / 'work').mkdir()
        (path / 'work/entry').write_text('owned')
        (path / 'work').chmod(0)
        install.clean_scratch(path)
        self.assertFalse(path.exists())


if __name__ == '__main__':
    unittest.main()
