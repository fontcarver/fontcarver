"""Remove image loaders the editor never uses, and only what followed them in."""

from pathlib import Path
import shutil
import struct
import sys
import tempfile
import unittest
import unittest.mock

sys.path.insert(0, str(Path(__file__).resolve().parent))

import appimage_trim as trim


def elf(path: Path, needed: list[str]) -> None:
    """Write a minimal 64-bit ELF whose PT_DYNAMIC lists these DT_NEEDED names."""
    path.parent.mkdir(parents=True, exist_ok=True)
    strtab = b'\0' + b'\0'.join(name.encode() for name in needed) + b'\0'
    offsets, cursor = [], 1
    for name in needed:
        offsets.append(cursor)
        cursor += len(name) + 1
    entries = b''.join(struct.pack('<qQ', 1, offset) for offset in offsets)
    entries += struct.pack('<qQ', 5, 0)          # DT_STRTAB, patched below
    entries += struct.pack('<qQ', 0, 0)          # DT_NULL
    header_size, phentsize = 64, 56
    dyn_off = header_size + phentsize
    str_off = dyn_off + len(entries)
    entries = entries.replace(struct.pack('<qQ', 5, 0),
                              struct.pack('<qQ', 5, str_off), 1)
    header = bytearray(b'\x7fELF\x02\x01\x01' + bytes(9))
    header += struct.pack('<HHIQQQIHHHHHH', 3, 62, 1, 0, header_size, 0, 0,
                          header_size, phentsize, 1, 0, 0, 0)
    # One PT_DYNAMIC segment whose virtual address equals its file offset.
    phdr = struct.pack('<IIQQQQQQ', 2, 4, dyn_off, dyn_off, dyn_off,
                       len(entries), len(entries), 8)
    path.write_bytes(bytes(header) + phdr + entries + strtab)


def loaded_elf(path: Path, needed: list[str], bias: int = 0x200000) -> None:
    """An ELF whose addresses differ from its file offsets, as a real one's do.

    DT_STRTAB holds a virtual address. Reading it as a file offset works only
    while the two happen to coincide, which is what a hand-built fixture does
    by accident and a linker does not.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    strtab = b'\0' + b'\0'.join(name.encode() for name in needed) + b'\0'
    offsets, cursor = [], 1
    for name in needed:
        offsets.append(cursor)
        cursor += len(name) + 1
    header_size, phentsize, phnum = 64, 56, 2
    dyn_off = header_size + phentsize * phnum
    entries = b''.join(struct.pack('<qQ', 1, offset) for offset in offsets)
    str_off = dyn_off + len(entries) + 32
    entries += struct.pack('<qQ', 5, str_off + bias)   # DT_STRTAB as an address
    entries += struct.pack('<qQ', 0, 0)
    pad = b'\0' * (str_off - dyn_off - len(entries))
    header = bytearray(b'\x7fELF\x02\x01\x01' + bytes(9))
    header += struct.pack('<HHIQQQIHHHHHH', 3, 62, 1, 0, header_size, 0, 0,
                          header_size, phentsize, phnum, 0, 0, 0)
    body = entries + pad + strtab
    load = struct.pack('<IIQQQQQQ', 1, 4, 0, bias, bias,
                       dyn_off + len(body), dyn_off + len(body), 0x1000)
    dynamic = struct.pack('<IIQQQQQQ', 2, 4, dyn_off, dyn_off + bias, dyn_off + bias,
                          len(entries), len(entries), 8)
    path.write_bytes(bytes(header) + load + dynamic + body)


class NeededTests(unittest.TestCase):
    def test_a_string_table_address_is_translated_to_a_file_offset(self):
        with tempfile.TemporaryDirectory() as work:
            path = Path(work) / 'real.so'
            loaded_elf(path, ['libtiff.so.6', 'libjpeg.so.8'])
            self.assertEqual(trim.needed(path), ['libtiff.so.6', 'libjpeg.so.8'])

    def test_dt_needed_names_are_read_without_executing_the_file(self):
        with tempfile.TemporaryDirectory() as work:
            path = Path(work) / 'lib.so'
            elf(path, ['libtiff.so.6', 'libc.so.6'])
            self.assertEqual(trim.needed(path), ['libtiff.so.6', 'libc.so.6'])

    @unittest.skipUnless(shutil.which('objdump'), 'needs binutils')
    def test_real_system_libraries_agree_with_objdump(self):
        # The fixtures above are built by hand. This is the check that the
        # reader works on libraries a linker produced.
        import subprocess
        checked = 0
        for name in ('libtiff.so.6', 'libgtk-3.so.0', 'libcairo.so.2', 'libglib-2.0.so.0'):
            path = Path('/usr/lib/x86_64-linux-gnu') / name
            if not path.exists():
                continue
            listing = subprocess.run(['objdump', '-p', str(path)],
                                     capture_output=True, text=True).stdout
            expected = {line.split()[1] for line in listing.splitlines()
                        if line.strip().startswith('NEEDED')}
            with self.subTest(library=name):
                self.assertEqual(set(trim.needed(path)), expected)
            checked += 1
        if not checked:
            self.skipTest('no reference libraries on this host')

    def test_a_file_that_is_not_an_elf_has_no_dependencies(self):
        with tempfile.TemporaryDirectory() as work:
            path = Path(work) / 'notes.txt'
            path.write_text('hello')
            self.assertEqual(trim.needed(path), [])


class TrimTests(unittest.TestCase):
    def payload(self, work: Path) -> Path:
        root = work / 'squashfs-root'
        loaders = root / 'usr/lib/gdk-pixbuf-2.0/2.10.0/loaders'
        loaders.mkdir(parents=True)
        (root / 'usr/bin').mkdir(parents=True)
        elf(root / 'usr/bin/fontcarver-desktop', ['libgdk_pixbuf-2.0.so.0'])
        elf(root / 'usr/lib/libgdk_pixbuf-2.0.so.0', [])
        elf(loaders / 'libpixbufloader-svg.so', ['librsvg-2.so.2'])
        elf(root / 'usr/lib/librsvg-2.so.2', [])
        elf(loaders / 'libpixbufloader-tiff.so', ['libtiff.so.6'])
        elf(root / 'usr/lib/libtiff.so.6', ['libjbig.so.0'])
        elf(root / 'usr/lib/libjbig.so.0', [])
        elf(loaders / 'libpixbufloader-bmp.so', [])
        # Nothing names this one: a module the program loads by hand. It must
        # survive, because reachability cannot see such a reference at all.
        elf(root / 'usr/lib/gio/modules/libgiognutls.so', [])
        (root / 'usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache').write_text('stale')
        return root

    def test_only_the_kept_loaders_and_what_they_need_remain(self):
        with tempfile.TemporaryDirectory() as work:
            root = self.payload(Path(work))
            report = trim.trim(root, keep={'svg'})
            loaders = root / 'usr/lib/gdk-pixbuf-2.0/2.10.0/loaders'
            self.assertEqual(sorted(p.name for p in loaders.glob('*.so')),
                             ['libpixbufloader-svg.so'])
            self.assertTrue((root / 'usr/lib/librsvg-2.so.2').exists())
            self.assertFalse((root / 'usr/lib/libtiff.so.6').exists())
            self.assertFalse((root / 'usr/lib/libjbig.so.0').exists())
            self.assertIn('libjbig.so.0', ' '.join(report['removed_libraries']))

    def test_a_library_nothing_names_is_left_alone(self):
        # Removing every unreachable file would delete modules the program
        # opens by name at run time; only what the removed loaders brought in
        # may go.
        with tempfile.TemporaryDirectory() as work:
            root = self.payload(Path(work))
            trim.trim(root, keep={'svg'})
            self.assertTrue((root / 'usr/lib/gio/modules/libgiognutls.so').exists())

    def test_the_stale_loader_cache_is_removed_with_the_loaders(self):
        with tempfile.TemporaryDirectory() as work:
            root = self.payload(Path(work))
            trim.trim(root, keep={'svg'})
            self.assertFalse(
                (root / 'usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache').exists())

    def test_symlinks_left_pointing_at_a_removed_loader_go_with_it(self):
        # usr/lib carries a symlink per loader. Removing the loader and leaving
        # the link behind produced a payload the inspector refused outright:
        # "unsafe or broken symlink".
        with tempfile.TemporaryDirectory() as work:
            root = self.payload(Path(work))
            link = root / 'usr/lib/libpixbufloader-tiff.so'
            link.symlink_to('gdk-pixbuf-2.0/2.10.0/loaders/libpixbufloader-tiff.so')
            kept = root / 'usr/lib/libpixbufloader-svg.so'
            kept.symlink_to('gdk-pixbuf-2.0/2.10.0/loaders/libpixbufloader-svg.so')
            report = trim.trim(root, keep={'svg'})
            self.assertFalse(link.is_symlink())
            self.assertTrue(kept.is_symlink())
            self.assertIn('usr/lib/libpixbufloader-tiff.so', report['removed_links'])

    def test_the_report_names_what_still_holds_a_candidate_library_open(self):
        # When a library the removed loaders used survives, the report has to
        # say which surviving file still needs it, or the next question has no
        # evidence behind it.
        with tempfile.TemporaryDirectory() as work:
            root = self.payload(Path(work))
            elf(root / 'usr/lib/gtk-3.0/3.0.0/immodules/im-x.so', ['libtiff.so.6'])
            report = trim.trim(root, keep={'svg'})
            self.assertTrue((root / 'usr/lib/libtiff.so.6').exists())
            self.assertIn('libtiff.so.6', report['retained'])
            self.assertIn('im-x.so', ' '.join(report['retained']['libtiff.so.6']))

    def test_a_payload_without_a_loader_directory_is_refused(self):
        with tempfile.TemporaryDirectory() as work:
            root = Path(work) / 'squashfs-root'
            (root / 'usr/bin').mkdir(parents=True)
            with self.assertRaisesRegex(ValueError, 'loader directory'):
                trim.trim(root, keep={'svg'})

    def test_keeping_a_loader_that_is_not_there_is_refused(self):
        # A typo in the keep list would otherwise silently ship no loader.
        with tempfile.TemporaryDirectory() as work:
            root = self.payload(Path(work))
            with self.assertRaisesRegex(ValueError, 'jpeg'):
                trim.trim(root, keep={'svg', 'jpeg'})


class RepackTests(unittest.TestCase):
    def test_the_image_keeps_its_own_runtime_and_gains_the_trimmed_payload(self):
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as work:
            root = Path(work)
            artifact = root / 'FontCarver.AppImage'
            artifact.write_bytes(b'RUNTIME' + b'OLD-SQUASHFS')
            artifact.chmod(0o755)
            calls = []

            def fake_run(argv, **options):
                calls.append(argv)
                if '--appimage-offset' in argv:
                    return SimpleNamespace(returncode=0, stdout=b'7\n', stderr=b'')
                if '--appimage-extract' in argv:
                    payload = Path(options['cwd']) / 'squashfs-root'
                    loaders = payload / 'usr/lib/gdk-pixbuf-2.0/2.10.0/loaders'
                    loaders.mkdir(parents=True)
                    (payload / 'usr/bin').mkdir(parents=True)
                    elf(payload / 'usr/bin/fontcarver-desktop', [])
                    elf(loaders / 'libpixbufloader-svg.so', [])
                    elf(loaders / 'libpixbufloader-tiff.so', ['libtiff.so.6'])
                    elf(payload / 'usr/lib/libtiff.so.6', [])
                    return SimpleNamespace(returncode=0, stdout=b'', stderr=b'')
                if argv[0] == 'mksquashfs':
                    Path(argv[2]).write_bytes(b'NEW-SQUASHFS')
                    return SimpleNamespace(returncode=0, stdout=b'', stderr=b'')
                raise AssertionError(f'unexpected command {argv}')

            with unittest.mock.patch.object(trim.subprocess, 'run', side_effect=fake_run), \
                 unittest.mock.patch.object(trim, 'write_notices', return_value=[]):
                report = trim.repack(artifact, keep={'svg'})
            self.assertEqual(artifact.read_bytes(), b'RUNTIME' + b'NEW-SQUASHFS')
            self.assertTrue(artifact.stat().st_mode & 0o111)
            self.assertEqual(report['removed_loaders'], ['libpixbufloader-tiff.so'])
            self.assertIn('usr/lib/libtiff.so.6', report['removed_libraries'])

    def test_the_notice_is_written_into_the_payload_before_it_is_packed(self):
        # The gate requires the bundled-library notice inside the image, and
        # the extracted payload during a repack is the only place to put it.
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as work:
            artifact = Path(work) / 'FontCarver.AppImage'
            artifact.write_bytes(b'RUNTIME' + b'OLD')
            seen = {}

            def fake_run(argv, **options):
                if '--appimage-offset' in argv:
                    return SimpleNamespace(returncode=0, stdout=b'7\n', stderr=b'')
                if '--appimage-extract' in argv:
                    payload = Path(options['cwd']) / 'squashfs-root'
                    loaders = payload / 'usr/lib/gdk-pixbuf-2.0/2.10.0/loaders'
                    loaders.mkdir(parents=True)
                    elf(loaders / 'libpixbufloader-svg.so', [])
                    elf(loaders / 'libpixbufloader-bmp.so', [])
                    return SimpleNamespace(returncode=0, stdout=b'', stderr=b'')
                if argv[0] == 'mksquashfs':
                    # The notice has to exist by the time the payload is packed.
                    seen['notice_present'] = Path(argv[1]) in seen.get('noticed', [])
                    Path(argv[2]).write_bytes(b'NEW')
                    return SimpleNamespace(returncode=0, stdout=b'', stderr=b'')
                raise AssertionError(argv)

            def fake_notices(payload, repo_root):
                seen.setdefault('noticed', []).append(payload)
                return ['usr/lib/FontCarver/licenses/appimage/BUNDLED-LIBRARIES.txt']

            with unittest.mock.patch.object(trim.subprocess, 'run', side_effect=fake_run), \
                 unittest.mock.patch.object(trim, 'write_notices', fake_notices):
                report = trim.repack(artifact, keep={'svg'})
            self.assertTrue(seen['notice_present'], 'notice written after packing')
            self.assertIn('BUNDLED-LIBRARIES.txt', ' '.join(report['notices']))

    def test_an_offset_that_is_not_a_number_is_refused(self):
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as work:
            artifact = Path(work) / 'x.AppImage'
            artifact.write_bytes(b'x')
            with unittest.mock.patch.object(
                    trim.subprocess, 'run',
                    return_value=SimpleNamespace(returncode=0, stdout=b'not a number\n', stderr=b'')), \
                 unittest.mock.patch.object(trim, 'write_notices', return_value=[]):
                with self.assertRaisesRegex(ValueError, 'offset'):
                    trim.repack(artifact, keep={'svg'})


if __name__ == '__main__':
    unittest.main()
