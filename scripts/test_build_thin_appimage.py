"""Boundary tests for the host-library AppImage builder."""
import hashlib
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import build_thin_appimage as builder


class ThinAppImageTests(unittest.TestCase):
    def test_download_rejects_changed_upstream_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            with mock.patch('urllib.request.urlopen') as request:
                request.return_value.__enter__.return_value.read.return_value = b'changed'
                with self.assertRaisesRegex(ValueError, 'checksum'):
                    builder.fetch('https://example.invalid/source', '0' * 64,
                                  Path(directory) / 'source')

    def test_download_accepts_exact_source(self):
        with tempfile.TemporaryDirectory() as directory:
            with mock.patch('urllib.request.urlopen') as request:
                request.return_value.__enter__.return_value.read.return_value = b'approved'
                target = Path(directory) / 'source'
                builder.fetch('https://example.invalid/source',
                              hashlib.sha256(b'approved').hexdigest(), target)
                self.assertEqual(target.read_bytes(), b'approved')

    def test_runtime_requires_exact_dynamic_library_set(self):
        dynamic = '\n'.join(f'(NEEDED) Shared library: [{name}]'
                            for name in builder.RUNTIME_NEEDED)
        self.assertEqual(builder.needed_libraries(dynamic), builder.RUNTIME_NEEDED)
        with self.assertRaisesRegex(ValueError, 'dynamic libraries'):
            builder.needed_libraries(dynamic + '\n(NEEDED) Shared library: [libsurprise.so]')
        with self.assertRaisesRegex(ValueError, 'dynamic libraries'):
            builder.needed_libraries('(NEEDED) Shared library: [libc.so.6]')

    def test_runtime_rejects_embedded_library_archives(self):
        for name in ('libfuse3.a', 'libsquashfuse.a', 'libzstd.a', 'libc.a'):
            with self.subTest(name=name):
                with self.assertRaisesRegex(ValueError, 'static library'):
                    builder.check_link_map(f'LOAD /somewhere/{name}\n')
        builder.check_link_map('LOAD /usr/lib/libgcc.a\nLOAD /usr/lib/libc_nonshared.a\n')
        with self.assertRaisesRegex(ValueError, 'static library archive member'):
            builder.check_link_map('/usr/lib/libgcc.a(hidden.o)\n')

    def test_payload_rejects_extra_elf_and_shared_objects(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / 'usr/bin/fontcarver-desktop'
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b'\x7fELFapp')
            builder.check_thin_payload(root)
            extra = root / 'helper'
            extra.write_bytes(b'\x7fELFhelper')
            with self.assertRaisesRegex(ValueError, 'ELF'):
                builder.check_thin_payload(root)
            extra.unlink()
            (root / 'libtest.so.1').write_text('not even an ELF')
            with self.assertRaisesRegex(ValueError, 'shared library'):
                builder.check_thin_payload(root)

    def test_payload_rejects_external_or_broken_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            link = root / 'outside'
            link.symlink_to('/etc/passwd')
            with self.assertRaisesRegex(ValueError, 'symlink'):
                builder.check_thin_payload(root)
            link.unlink()
            link.symlink_to('outside')
            with self.assertRaisesRegex(ValueError, 'symlink'):
                builder.check_thin_payload(root)
            link.unlink()
            link.symlink_to('missing')
            with self.assertRaisesRegex(ValueError, 'symlink'):
                builder.check_thin_payload(root)

    def test_patch_fails_if_upstream_context_changes(self):
        with self.assertRaisesRegex(ValueError, 'source patch'):
            builder.patch_runtime('unknown upstream source')

    def test_launcher_preserves_spaces_arguments_and_own_appdir(self):
        import os
        import subprocess
        with tempfile.TemporaryDirectory(prefix='app with spaces ') as directory:
            root = Path(directory)
            binary = root / 'usr/bin/fontcarver-desktop'
            binary.parent.mkdir(parents=True)
            binary.write_text('#!/bin/sh\nprintf "%s\\n" "$APPDIR" "$@"\n')
            binary.chmod(0o755)
            launcher = root / 'AppRun'
            launcher.write_text(builder.APPRUN)
            launcher.chmod(0o755)
            output = subprocess.check_output([str(launcher), 'one two', '--three'],
                                             env={**os.environ, 'APPDIR': '/wrong'})
            self.assertEqual(output.decode().splitlines(), [str(root), 'one two', '--three'])


if __name__ == '__main__':
    unittest.main()
