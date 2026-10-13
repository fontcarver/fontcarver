#!/usr/bin/env python3
"""Drop gdk-pixbuf image loaders the editor never uses, and what followed them.

The AppImage bundler copies every gdk-pixbuf loader present on the build host.
FontCarver needs none of them but SVG: PNG and JPEG are built into gdk-pixbuf
itself, and WebKit decodes page images on its own. The TIFF loader is the reason
the payload carries libtiff, and libtiff is the reason it carries libjbig, which
is GPL-2+ with no library exception — the one real copyleft library in the image
(see docs/appimage-license-determinations.md).

Only what the removed loaders brought in is removed with them. A library that
nothing in the payload names is left alone: modules opened by name at run time
have no incoming reference to find, so "unreachable" never means "unused".

    python3 scripts/appimage_trim.py --payload PATH/squashfs-root
"""

from __future__ import annotations

import argparse
import collections
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile

from appimage_inventory import write_notices

ROOT = Path(__file__).resolve().parents[1]
LOADER_DIR = 'usr/lib/gdk-pixbuf-2.0/2.10.0/loaders'
LOADER_CACHE = 'usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache'
KEEP = {'svg'}

_DT_NULL, _DT_NEEDED, _DT_STRTAB = 0, 1, 5
_PT_LOAD, _PT_DYNAMIC = 1, 2


def needed(path: Path) -> list[str]:
    """Read an ELF's DT_NEEDED names, without executing or linking anything."""
    try:
        data = path.read_bytes()
    except OSError:
        return []
    if len(data) < 64 or data[:4] != b'\x7fELF' or data[4] != 2 or data[5] != 1:
        return []  # 64-bit little-endian only; anything else is not ours to read.
    phoff, = struct.unpack_from('<Q', data, 32)
    phentsize, phnum = struct.unpack_from('<HH', data, 54)
    dynamic, loads = None, []
    for index in range(phnum):
        base = phoff + index * phentsize
        if base + 56 > len(data):
            return []
        p_type, = struct.unpack_from('<I', data, base)
        p_offset, p_vaddr, _p_paddr, p_filesz = struct.unpack_from('<QQQQ', data, base + 8)
        if p_type == _PT_DYNAMIC and dynamic is None:
            dynamic = (p_offset, p_filesz)
        elif p_type == _PT_LOAD:
            loads.append((p_vaddr, p_offset, p_filesz))
    if not dynamic:
        return []
    offset, size = dynamic

    def file_offset(address: int) -> int | None:
        # DT_STRTAB holds a virtual address. Only the segment that maps it can
        # say where it sits in the file, and in a real library the two differ.
        for vaddr, voffset, filesz in loads:
            if vaddr <= address < vaddr + filesz:
                return voffset + (address - vaddr)
        return None
    entries, cursor = [], offset
    while cursor + 16 <= min(offset + size, len(data)):
        tag, value = struct.unpack_from('<qQ', data, cursor)
        cursor += 16
        if tag == _DT_NULL:
            break
        entries.append((tag, value))
    address = next((value for tag, value in entries if tag == _DT_STRTAB), None)
    if address is None:
        return []
    strtab = file_offset(address)
    if strtab is None:
        # No PT_LOAD covers it; the only remaining reading is a direct offset,
        # which is what a fixture built by hand produces.
        strtab = address if not loads else None
    if strtab is None:
        return []
    names = []
    for tag, value in entries:
        if tag != _DT_NEEDED:
            continue
        start = strtab + value
        end = data.find(b'\0', start)
        if 0 <= start < len(data) and end > start:
            names.append(data[start:end].decode('utf-8', 'replace'))
    return names


def _libraries(root: Path) -> dict[str, Path]:
    """Map every payload library's file name to its path, for soname lookup."""
    found: dict[str, Path] = {}
    for path in root.rglob('*'):
        if path.is_file() and not path.is_symlink() and '.so' in path.name:
            found.setdefault(path.name, path)
    return found


def _reachable(roots: list[Path], libraries: dict[str, Path]) -> set[Path]:
    """Every payload library reachable from these files by DT_NEEDED."""
    seen: set[Path] = set()
    queue = collections.deque(roots)
    while queue:
        current = queue.popleft()
        for name in needed(current):
            target = libraries.get(name)
            if target and target not in seen:
                seen.add(target)
                queue.append(target)
    return seen


def trim(root: Path, keep: set[str] | None = None) -> dict:
    """Remove the unkept loaders, then the libraries only they reached."""
    keep = set(keep or KEEP)
    loaders = root / LOADER_DIR
    if not loaders.is_dir():
        raise ValueError(f'no gdk-pixbuf loader directory at {LOADER_DIR}')
    present = {path.name.removeprefix('libpixbufloader-').removesuffix('.so'): path
               for path in sorted(loaders.glob('libpixbufloader-*.so'))}
    missing = sorted(keep - set(present))
    if missing:
        raise ValueError(f'loader(s) to keep are not in the payload: {", ".join(missing)}')
    doomed = [path for name, path in present.items() if name not in keep]
    libraries = _libraries(root)
    flat = root / 'usr/lib'
    # Reachability has to start from entry points, not from every file: a
    # library counted as its own root would keep whatever it names alive. An
    # entry point is anything that is not a plain library in usr/lib — the
    # executables, the launcher, and every module directory, whose contents are
    # opened by name at run time and so are never named by anything.
    entries = [path for path in root.rglob('*')
               if path.is_file() and not path.is_symlink() and path not in doomed
               and not (path.parent == flat and '.so' in path.name)]
    # Only what the removed loaders brought in: a library nothing names at all
    # is unreachable both before and after, and stays.
    from_doomed = _reachable(doomed, libraries)
    kept_reach = _reachable(entries, libraries)
    lost = from_doomed - kept_reach
    # A library the removed loaders used that something else still needs. Name
    # what holds it open, so the next question starts from evidence.
    retained: dict[str, list[str]] = {}
    for library in sorted(from_doomed & kept_reach):
        holders = sorted(str(entry.relative_to(root)) for entry in entries
                         if library.name in needed(entry))
        retained[library.name] = holders or ['(reached indirectly)']
    for path in doomed:
        path.unlink()
    for path in sorted(lost):
        path.unlink(missing_ok=True)
    # usr/lib carries a symlink per loader. One left pointing at a file this
    # trim removed makes the whole payload unreadable to the inspector.
    removed_links = []
    for path in sorted(root.rglob('*')):
        if path.is_symlink() and not path.exists():
            removed_links.append(str(path.relative_to(root)))
            path.unlink()
    cache = root / LOADER_CACHE
    # The cache names the files that were here. gdk-pixbuf rebuilds it, and a
    # stale one would point at loaders this payload no longer carries.
    if cache.is_file():
        cache.unlink()
    return {'kept_loaders': sorted(keep),
            'removed_loaders': sorted(path.name for path in doomed),
            'removed_libraries': sorted(str(path.relative_to(root)) for path in lost),
            'removed_links': removed_links,
            'retained': retained}


def _run(argv: list[str], cwd: Path | None = None) -> bytes:
    result = subprocess.run(argv, cwd=cwd, capture_output=True, timeout=1800)
    if result.returncode:
        detail = (result.stderr or result.stdout).decode(errors='replace')[-2000:]
        raise ValueError(f'{argv[0]} exited {result.returncode}: {detail}')
    return result.stdout


def repack(artifact: Path, keep: set[str] | None = None) -> dict:
    """Trim the payload and rebuild the image behind its own runtime.

    The runtime prefix is reused byte for byte: it is the image's own ELF, and
    rebuilding it would be a different artifact. Only the squashfs behind it is
    replaced. gzip keeps the result readable by every AppImage runtime.
    """
    artifact = artifact.resolve(strict=True)
    raw = _run([str(artifact), '--appimage-offset']).decode().strip()
    if not raw.isdigit():
        raise ValueError(f'unusable AppImage runtime offset: {raw!r}')
    offset = int(raw)
    runtime = artifact.read_bytes()[:offset]
    if len(runtime) != offset:
        raise ValueError('AppImage is shorter than its own runtime offset')
    with tempfile.TemporaryDirectory(dir=artifact.parent) as work:
        scratch = Path(work)
        _run([str(artifact), '--appimage-extract'], cwd=scratch)
        payload = scratch / 'squashfs-root'
        report = trim(payload, keep)
        # The gate looks for the bundled-library notice inside the image, so it
        # has to be in the payload before the payload becomes one.
        report['notices'] = write_notices(payload, ROOT)
        image = scratch / 'payload.squashfs'
        _run(['mksquashfs', str(payload), str(image), '-root-owned', '-noappend',
              '-comp', 'gzip', '-b', '131072', '-no-progress', '-quiet'])
        artifact.write_bytes(runtime + image.read_bytes())
    artifact.chmod(artifact.stat().st_mode | 0o111)
    report['artifact'] = artifact.name
    report['runtime_bytes'] = offset
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--payload', type=Path, required=True)
    parser.add_argument('--keep', default=','.join(sorted(KEEP)),
                        help='comma-separated loader names to keep')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args(argv)
    report = trim(args.payload.resolve(strict=True),
                  {name.strip() for name in args.keep.split(',') if name.strip()})
    print(f'kept {report["kept_loaders"]}, removed {len(report["removed_loaders"])} loaders '
          f'and {len(report["removed_libraries"])} libraries')
    for name in report['removed_libraries']:
        print(f'  - {name}')
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
