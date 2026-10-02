#!/usr/bin/env python3
"""Build a thin Linux x86_64 AppImage using dynamic host libraries.

This is an explicitly host-dependent alternative to Tauri's library-bundling
AppImage. It requires the normal desktop prerequisites plus squashfuse 0.5.0
and FUSE 3. It never copies a host shared library into the image. Build tools:
gcc, binutils, pkg-config, squashfs-tools, libsquashfuse-dev, libfuse3-dev.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
PROFILE = 'fontcarver-host-libs-v1'
BUNDLE_PREFIX = b'__TAURI_BUNDLE_TYPE_VAR_'
COMMIT = '8f39b89e2ac31e1640b3d3f7e9a5108e6ce805fa'
BASE = f'https://raw.githubusercontent.com/AppImage/type2-runtime/{COMMIT}/'
SOURCES = {
    'runtime.upstream.c': (BASE + 'src/runtime/runtime.c',
        'e90a7685e1e11cd422bbc9cc0032c975ef9ca998e3c3708050c1c5134286f618'),
    'sections.upstream.ld': (BASE + 'src/runtime/data_sections.ld',
        '3d62a1eaa49c62569927a668d1c62d4b6ebf94a14db9b39ef99ea3d14cd79ab2'),
    'LICENSE.runtime': (BASE + 'LICENSE',
        'aa154fc9070614bbe7921f89db11efd1dba7a1f3a41685958110e2230f9c0ca1'),
    'fuseprivate.h': ('https://raw.githubusercontent.com/vasi/squashfuse/0.5.0/fuseprivate.h',
        '47445b689ffbcaa79527edea87edff823a7b33186b16929ec53b2eacb6b52c08'),
    'LICENSE.squashfuse': ('https://raw.githubusercontent.com/vasi/squashfuse/0.5.0/LICENSE',
        '9e909cc8a8ba27b1a649c964cf9eda37de911f3138c2b64e6d2f01703904ac13'),
    'HOST-LGPL-2.1.txt': ('https://raw.githubusercontent.com/libfuse/libfuse/'
        '3f65bb699cd2f0afe2b9519c39d29922229f2168/LGPL2.txt',
        'dc626520dcd53a22f727af3ee42c770e56c97a64fe3adb063799d8ab032fe551'),
}
APPROVED_INPUTS = {name: details[1] for name, details in SOURCES.items()}
APPROVED_INPUTS.update({
    'runtime.c': '1174937b9d45acffa9cc4db52f9e085bcff3d469f7217313a709be093e9da0ba',
    'sections.ld': 'c2026530788f74487818c86b41f020b66b16faa1b8ef6f73fd9983b7549c3e03',
    'README.txt': '2f747412f5ac4046023eb03cea583031d822fc17aa662e1099b4c63beb7797f9',
})
RUNTIME_NEEDED = {'libsquashfuse.so.0', 'libsquashfuse_ll.so.0',
                  'libfuse3.so.3', 'libc.so.6'}
REQUIRED_NEEDED = RUNTIME_NEEDED
APPRUN = '''#!/bin/sh
set -eu
appdir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export APPDIR="$appdir"
exec "$appdir/usr/bin/fontcarver-desktop" "$@"
'''
APP_RUN = APPRUN.encode()
NOTICE = '''FontCarver thin AppImage: host-library build

The runtime is built from the pinned MIT/public-domain type2-runtime source
included here, with bounded I/O/path checks and corrected dynamic-build help.
LICENSE.runtime is the unmodified upstream license; its trailing static-build
dependency list describes upstream binaries, not this variant. No musl, FUSE,
squashfuse, zlib, zstd or GTK/WebKit library is copied into this image. Squashfuse
headers used at build time are BSD-2-Clause (LICENSE.squashfuse).

This runtime uses system libfuse3 under LGPL-2.1; see HOST-LGPL-2.1.txt.
Libfuse: Copyright (C) 2001-2007 Miklos Szeredi and contributors.
ABI-compatible host replacements remain supported through normal dynamic linking.
Host GTK/WebKit and other library notices accompany the distribution's packages;
consult their installed copyright/license documents. Including this license text
does not change FontCarver's MIT OR Apache-2.0 license.

This candidate requires a compatible Linux x86_64 host, tested on Ubuntu 24.04:
libsquashfuse0 (= 0.5.0-2build1), libfuse3-3, fuse3 and the GTK/WebKit/other
runtime dependencies listed by its input deb. Normal launch needs working FUSE;
--appimage-extract-and-run avoids mounting but still needs these host libraries.
This is not a self-contained, universally portable Linux package.

BUILD.json, compiler link map, source and linker script record the recipe and
exact bytes. The receipt is build evidence, not a signature or protection from
deliberate forgery. No installation, signing or publishing is performed here.
'''


def digest(path: Path) -> str:
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def fetch(url: str, expected: str, output: Path) -> None:
    """Accept source only after its reviewed content checksum matches."""
    with urllib.request.urlopen(url, timeout=60) as response:
        content = response.read(1_000_001)
    if len(content) > 1_000_000 or hashlib.sha256(content).hexdigest() != expected:
        raise ValueError(f'source checksum mismatch: {url}')
    output.write_bytes(content)


def replace_once(source: str, old: str, new: str) -> str:
    if source.count(old) != 1:
        raise ValueError('upstream source patch context changed')
    return source.replace(old, new, 1)


def patch_runtime(source: str) -> str:
    """Keep upstream behavior, fixing GCC-reported bounded I/O and path checks."""
    source = replace_once(source, '''    fseek(f, offset, SEEK_SET);

    char* buffer = calloc(length + 1, sizeof(char));
    fread(buffer, length, sizeof(char), f);
''', '''    if (fseek(f, offset, SEEK_SET) != 0 || length == ULONG_MAX) {
        fclose(f);
        return NULL;
    }
    char* buffer = calloc(length + 1, sizeof(char));
    if (buffer == NULL || fread(buffer, 1, length, f) != length) {
        free(buffer);
        fclose(f);
        return NULL;
    }
''')
    old_readlink = 'readlink(appimage_path, fullpath, sizeof(fullpath));'
    if source.count(old_readlink) != 3:
        raise ValueError('upstream source patch readlink context changed')
    source = source.replace(old_readlink, 'readlink(appimage_path, fullpath, sizeof(fullpath) - 1);')
    source = replace_once(source, '        sprintf(portable_dir, "%s.%s", fullpath, name);',
                          '''        int written = snprintf(portable_dir, sizeof(portable_dir),
                               "%s.%s", fullpath, name);
        if (written < 0 || (size_t)written >= sizeof(portable_dir)) {
            fprintf(stderr, "Portable directory path is too long\\n");
            exit(EXIT_FAILURE);
        }''')
    source = replace_once(source, '        read(keepalive_pipe[0], &c, 1);',
                          '''        ssize_t keepalive_result;
        do {
            keepalive_result = read(keepalive_pipe[0], &c, 1);
        } while (keepalive_result < 0 && errno == EINTR);
        if (keepalive_result < 0) perror("read keepalive");''')
    start = source.index('            "  * musl libc, licensed under the terms of')
    end = source.index('            "Please see https://github.com/AppImage/type2-runtime', start)
    return source[:start] + '''            "  This variant dynamically links host FUSE 3 and squashfuse.\\n"
            "  Host libraries are not bundled. See licenses/thin-appimage.\\n"
''' + source[end:]


def patch_linker(source: str) -> str:
    # The upstream marker advertises a static binary, so it must be absent here.
    return re.sub(r'  /\* undocumented marker.*?\n  }\n', '', source, count=1, flags=re.S)


def needed_libraries(output: str) -> set[str]:
    libraries = set(re.findall(r'\(NEEDED\).*?\[(.*?)\]', output))
    if libraries != RUNTIME_NEEDED:
        raise ValueError(f'unexpected runtime dynamic libraries: {sorted(libraries)}')
    if '(RPATH)' in output or '(RUNPATH)' in output:
        raise ValueError('runtime must not retain a build-machine library search path')
    return libraries


def check_link_map(text: str) -> None:
    # Compiler startup support is the same host toolchain infrastructure used by
    # ordinary ELF applications. No library archive payload may be added here.
    archives = set(re.findall(r'[^\s()]+\.a(?=[\s(]|$)', text))
    unexpected = {Path(name).name for name in archives} - {'libgcc.a', 'libc_nonshared.a'}
    if unexpected:
        raise ValueError(f'unexpected static library in runtime: {sorted(unexpected)}')
    if re.search(r'\.a\s*\(', text):
        raise ValueError('a static library archive member was linked into the runtime')


def check_thin_payload(root: Path) -> None:
    for path in root.rglob('*'):
        if path.is_symlink():
            try:
                resolved = path.resolve(strict=True)
            except (OSError, RuntimeError) as error:
                raise ValueError(f'broken or cyclic payload symlink: {path}') from error
            if not resolved.is_relative_to(root.resolve()):
                raise ValueError(f'escaping payload symlink: {path}')
        if not path.is_dir() and not path.is_file():
            raise ValueError(f'unsupported payload file type: {path}')
        if path.is_file():
            if re.search(r'\.so(?:\.|$)', path.name):
                raise ValueError(f'bundled shared library is forbidden: {path}')
            with path.open('rb') as stream:
                elf = stream.read(4) == b'\x7fELF'
            if elf and path != root / 'usr/bin/fontcarver-desktop':
                raise ValueError(f'extra ELF payload is forbidden: {path}')
            if (path.stat().st_mode & 0o111
                    and path not in (root / 'AppRun', root / 'usr/bin/fontcarver-desktop')):
                raise ValueError(f'extra executable payload is forbidden: {path}')


def patch_bundle_type(binary: Path) -> dict:
    """Change only Tauri's unique DEB marker in the extracted ELF to APP.

    tauri-utils 2.9.3 platform::bundle_type maps APP to AppImage. Tauri's own
    bundler applies the same fixed-width marker transformation. The input deb
    remains untouched; the receipt binds both ELF hashes and the three-byte edit.
    """
    if binary.is_symlink() or not binary.is_file():
        raise ValueError('bundle conversion requires a regular application ELF')
    source = binary.read_bytes()
    if not source.startswith(b'\x7fELF'):
        raise ValueError('bundle conversion requires an ELF application')
    if source.count(BUNDLE_PREFIX) != 1:
        raise ValueError('application bundle marker is missing, duplicated or conflicting')
    offset = source.index(BUNDLE_PREFIX) + len(BUNDLE_PREFIX)
    if source[offset:offset + 3] != b'DEB':
        raise ValueError('application bundle marker must be DEB before AppImage conversion')
    with binary.open('r+b') as stream:
        stream.seek(offset)
        stream.write(b'APP')
    return {'source_sha256': hashlib.sha256(source).hexdigest(),
            'appimage_sha256': digest(binary), 'marker_offset': offset,
            'before': 'DEB', 'after': 'APP'}


def run(command: list[str], *, cwd: Path, env: dict[str, str]) -> str:
    print('+ ' + ' '.join(command), flush=True)
    result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, timeout=300)
    if result.returncode:
        raise ValueError(f'{command[0]} failed ({result.returncode}):\n{result.stderr[-5000:]}')
    if result.stderr:
        print(result.stderr, end='', file=sys.stderr)
    return result.stdout


def build_runtime(scratch: Path, notices: Path, sysroot: Path, env: dict[str, str]) -> tuple[Path, dict]:
    notices.mkdir(parents=True)
    for name, (url, checksum) in SOURCES.items():
        fetch(url, checksum, notices / name)
    (notices / 'runtime.c').write_text(patch_runtime((notices / 'runtime.upstream.c').read_text()))
    (notices / 'sections.ld').write_text(patch_linker((notices / 'sections.upstream.ld').read_text()))
    (notices / 'README.txt').write_text(NOTICE)
    for name, expected in APPROVED_INPUTS.items():
        if digest(notices / name) != expected:
            raise ValueError(f'patched source checksum mismatch: {name}')
    includes = scratch / 'include/squashfuse'
    includes.mkdir(parents=True)
    shutil.copyfile(notices / 'fuseprivate.h', includes / 'fuseprivate.h')
    pkg_env = dict(env, PKG_CONFIG_PATH=str(sysroot / 'usr/lib/x86_64-linux-gnu/pkgconfig'))
    version = run(['pkg-config', '--modversion', 'squashfuse'], cwd=scratch, env=pkg_env).strip()
    if version != '0.5.0':
        raise ValueError(f'reviewed squashfuse headers must be 0.5.0, found {version}')
    runtime = scratch / 'runtime'
    command = ['gcc', '-std=gnu99', '-Os', '-D_FILE_OFFSET_BITS=64',
               f'-DGIT_COMMIT="{COMMIT}-host-libs"', '-ffunction-sections', '-fdata-sections',
               '-Wl,--gc-sections', '-Wall', '-Werror', '-fPIE', '-pie',
               '-Wl,-z,relro,-z,now', '-T', str(notices / 'sections.ld'),
               '-I', str(scratch / 'include'), '-I', str(sysroot / 'usr/include/squashfuse'),
               '-I', str(sysroot / 'usr/include'), '-I', str(sysroot / 'usr/include/fuse3'),
               str(notices / 'runtime.c'), '-L', str(sysroot / 'usr/lib/x86_64-linux-gnu'),
               '-Wl,-Bdynamic', '-l:libsquashfuse.so.0', '-l:libsquashfuse_ll.so.0',
               '-l:libfuse3.so.3', '-Wl,-Map,' + str(notices / 'runtime.map'), '-o', str(runtime)]
    run(command, cwd=scratch, env=env)
    check_link_map((notices / 'runtime.map').read_text())
    run(['strip', '--strip-unneeded', str(runtime)], cwd=scratch, env=env)
    with runtime.open('r+b') as stream:
        stream.seek(8)
        stream.write(b'AI\x02')
    needed = needed_libraries(run(['readelf', '-d', str(runtime)], cwd=scratch, env=env))
    return runtime, {'bytes': runtime.stat().st_size, 'sha256': digest(runtime), 'needed': sorted(needed),
                     'compiler': run(['gcc', '--version'], cwd=scratch, env=env).splitlines()[0],
                     'command': command}


def build(deb: Path, output: Path, scratch: Path, sysroot: Path, env: dict[str, str]) -> dict:
    from check_release_bundle import inspect_payload
    if deb.is_symlink() or not deb.is_file() or deb.suffix != '.deb':
        raise ValueError('input must be a regular deb file')
    appdir = scratch / 'AppDir'
    run(['dpkg-deb', '--extract', str(deb), str(appdir)], cwd=scratch, env=env)
    check_thin_payload(appdir)
    inspect_payload(appdir, 'deb', {'Package': 'font-carver'})
    application = patch_bundle_type(appdir / 'usr/bin/fontcarver-desktop')
    (appdir / 'AppRun').write_bytes(APP_RUN)
    (appdir / 'AppRun').chmod(0o755)
    desktop = next((appdir / 'usr/share/applications').glob('*.desktop'))
    shutil.copyfile(desktop, appdir / desktop.name)
    icons = sorted((appdir / 'usr/share/icons').rglob('*.png'))
    if not icons:
        raise ValueError('deb has no application PNG icon')
    shutil.copyfile(icons[-1], appdir / icons[-1].name)
    shutil.copyfile(icons[-1], appdir / '.DirIcon')
    notices = appdir / 'usr/lib/FontCarver/licenses/thin-appimage'
    runtime, runtime_info = build_runtime(scratch, notices, sysroot, env)
    receipt = {'schema_version': 1, 'profile': PROFILE, 'source_commit': COMMIT,
               'runtime': runtime_info, 'application': application, 'deb_sha256': digest(deb),
               'deb_depends': run(['dpkg-deb', '-f', str(deb), 'Depends'], cwd=scratch, env=env).strip(),
               'apprun_sha256': hashlib.sha256(APP_RUN).hexdigest(),
               'inputs': {p.name: digest(p) for p in notices.iterdir() if p.is_file()},
               'mksquashfs': run(['mksquashfs', '-version'], cwd=scratch, env=env).splitlines()[0]}
    (notices / 'BUILD.json').write_text(json.dumps(receipt, indent=2) + '\n')
    check_thin_payload(appdir)
    filesystem = scratch / 'payload.squashfs'
    run(['mksquashfs', str(appdir), str(filesystem), '-noappend', '-comp', 'gzip',
         '-all-root', '-no-xattrs', '-processors', '2', '-mkfs-time', '0', '-all-time', '0'],
        cwd=scratch, env=env)
    output.parent.mkdir(parents=True, exist_ok=True)
    # Atomic replacement leaves an existing candidate intact if a build fails.
    with tempfile.NamedTemporaryFile(dir=output.parent, prefix='.thin-', delete=False) as target:
        partial = Path(target.name)
        try:
            for source in (runtime, filesystem):
                with source.open('rb') as stream:
                    shutil.copyfileobj(stream, target)
            target.flush()
            partial.chmod(0o755)
            partial.replace(output)
        finally:
            partial.unlink(missing_ok=True)
    return {'artifact': str(output), 'sha256': digest(output), 'runtime': runtime_info}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--deb', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--sysroot', type=Path, default=Path('/'),
                        help='optional extracted build-dependency root; nothing is copied from it')
    args = parser.parse_args()
    if platform.system() != 'Linux' or platform.machine() != 'x86_64':
        parser.error('the reviewed thin candidate supports Linux x86_64 only')
    task = ROOT / '.scratch/p5f-w3-thin-appimage'
    task.mkdir(parents=True, exist_ok=True)
    try:
        with tempfile.TemporaryDirectory(prefix='build-', dir=task) as temporary:
            scratch = Path(temporary).resolve()
            env = dict(os.environ, TMPDIR=str(scratch))
            result = build(args.deb.absolute(), args.output.absolute(), scratch, args.sysroot.resolve(), env)
        print(json.dumps(result, indent=2))
        return 0
    except (ValueError, OSError, subprocess.TimeoutExpired) as error:
        print(f'Thin AppImage build failed: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
