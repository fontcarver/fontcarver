#!/usr/bin/env python3
"""Inventory an extracted AppImage and its separate runtime without running it.

Example (extract a trusted local build first):
  python3 scripts/appimage_inventory.py --payload PATH/squashfs-root \
    --appimage PATH/FontCarver.AppImage --runtime-bytes OFFSET --output audit.json

Debian package ownership is a current-host hint, not proof of the image's build
provenance. Every directory, symlink and regular file is recorded; regular files
are hashed. This diagnostic command never grants redistribution approval.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys

COPYLEFT = re.compile(r'\b(?:A?GPL|LGPL|MPL|SSPL|CDDL|EUPL)(?:\b|[-\d])', re.I)


def sha256(path: Path) -> str:
    """Hash a regular file without loading the complete payload into memory."""
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def elf_machine(path: Path) -> int | None:
    """Read ELF architecture directly, without executing a candidate library."""
    with path.open('rb') as stream:
        header = stream.read(20)
    if len(header) < 20 or header[:4] != b'\x7fELF' or header[5] not in (1, 2):
        return None
    return int.from_bytes(header[18:20], 'little' if header[5] == 1 else 'big')


def payload_files(root: Path) -> list[dict]:
    """Account for the entire tree and reject paths that escape its boundary."""
    root = root.resolve(strict=True)
    if not root.is_dir():
        raise ValueError('payload must be a directory')
    rows = []
    for path in sorted(root.rglob('*')):
        info = path.lstat()
        row = {'path': path.relative_to(root).as_posix(), 'mode': oct(stat.S_IMODE(info.st_mode))}
        if stat.S_ISLNK(info.st_mode):
            try:
                resolved = path.resolve(strict=True)
            except (OSError, RuntimeError) as error:
                raise ValueError(f'unsafe or broken symlink: {row["path"]}') from error
            if not resolved.is_relative_to(root):
                raise ValueError(f'unsafe or broken symlink: {row["path"]}')
            row.update(type='symlink', target=os.readlink(path))
        elif stat.S_ISDIR(info.st_mode):
            row['type'] = 'directory'
        elif stat.S_ISREG(info.st_mode):
            row.update(type='file', bytes=info.st_size, sha256=sha256(path))
            machine = elf_machine(path)
            if machine is not None:
                row['elf_machine'] = machine
        else:
            raise ValueError(f'unsupported payload file type: {row["path"]}')
        rows.append(row)
    return rows


def runtime_record(artifact: Path, offset: int) -> dict:
    """Record the executable prefix separately from the SquashFS payload."""
    if offset < 20 or offset > min(artifact.stat().st_size - 4, 64 * 1024 * 1024):
        raise ValueError('invalid runtime offset (expected ELF prefix of at most 64 MiB)')
    with artifact.open('rb') as stream:
        prefix = stream.read(offset)
        if stream.read(4) != b'hsqs':
            raise ValueError('runtime offset does not point to a SquashFS header')
    if prefix[:4] != b'\x7fELF':
        raise ValueError('runtime prefix is not ELF')
    origins = sorted(set(value.decode('ascii') for value in re.findall(
        rb'https://github\.com/AppImage/[A-Za-z0-9_-]+/commit/[0-9a-f]{7,40}', prefix)))
    return {'bytes': offset, 'sha256': hashlib.sha256(prefix).hexdigest(),
            'embedded_source_references': origins, 'license_status': 'unreviewed'}


def license_summary(text: str) -> dict:
    """Treat package-wide labels as review hints, never per-file decisions."""
    labels = sorted(set(re.findall(r'^License:\s*([^\n]+)', text, re.M)))
    return {'labels': labels, 'copyleft_mentioned': bool(COPYLEFT.search('\n'.join(labels))),
            'classification': 'needs-file-review' if labels else 'unknown'}


def host_inventory(names: set[str]) -> tuple[dict, dict]:
    """Index current dpkg ownership for relevant basenames, including multiarch."""
    result = subprocess.run(
        ['dpkg-query', '-W', '-f=${binary:Package}\t${Version}\t${source:Package}\t${source:Version}\n'],
        capture_output=True, text=True, check=True, timeout=30)
    packages = {}
    for line in result.stdout.splitlines():
        name, version, source, source_version = line.split('\t')
        packages[name] = {'version': version, 'source_package': source, 'source_version': source_version}
    owners: dict[str, list[tuple[str, str]]] = {}
    for listing in sorted(Path('/var/lib/dpkg/info').glob('*.list')):
        package = listing.name[:-5]
        if package not in packages:
            continue
        for path in listing.read_text(encoding='utf-8', errors='replace').splitlines():
            name = Path(path).name
            if name in names:
                owners.setdefault(name, []).append((package, path))
    return owners, packages


def annotate_host_candidates(rows: list[dict], owners: dict, packages: dict,
                             host_root: Path = Path('/')) -> None:
    """Identify ELF candidates conservatively; modified files stay unverified."""
    for row in rows:
        if 'elf_machine' not in row:
            continue
        matches = []
        for package, host_path in owners.get(Path(row['path']).name, []):
            candidate = host_root / host_path.lstrip('/')
            if not candidate.is_file() or elf_machine(candidate) != row['elf_machine']:
                continue
            digest = sha256(candidate)
            identical = digest == row['sha256']
            matches.append({'package': package, **packages[package], 'path': host_path,
                            'sha256': digest, 'content_identical': identical,
                            'match': 'exact-content' if identical else 'name-and-architecture-only'})
        row['host_candidates'] = matches


def package_notices(root: Path, rows: list[dict], packages: dict) -> list[dict]:
    """Record current-host copyright hashes and notices actually in the image."""
    result = []
    names = sorted({candidate['package'] for row in rows
                    for candidate in row.get('host_candidates', [])})
    for name in names:
        base = name.split(':', 1)[0]
        copyright_path = Path('/usr/share/doc') / base / 'copyright'
        source = packages.get(name, {})
        entry = {'package': name, 'host_copyright_path': str(copyright_path),
                 'version': source.get('version', ''),
                 'source_package': source.get('source_package', ''),
                 'source_version': source.get('source_version', '')}
        if copyright_path.is_file():
            entry['host_copyright_sha256'] = sha256(copyright_path)
            entry.update(license_summary(copyright_path.read_text(encoding='utf-8', errors='replace')))
        else:
            entry.update(license_summary(''))
        shipped = root / 'usr/share/doc' / base / 'copyright'
        # payload_files already checked that every symlink stays inside root.
        entry['shipped_copyright'] = shipped.relative_to(root).as_posix() if shipped.is_file() else None
        if shipped.is_file():
            entry['shipped_copyright_sha256'] = sha256(shipped)
        result.append(entry)
    return result


def inventory(artifact: Path, payload: Path, offset: int) -> dict:
    """Generate complete diagnostic evidence with an explicit blocked decision."""
    rows = payload_files(payload)
    owners, packages = host_inventory({Path(row['path']).name for row in rows if 'elf_machine' in row})
    annotate_host_candidates(rows, owners, packages)
    notices = package_notices(payload, rows, packages)
    libraries = [row for row in rows if 'elf_machine' in row and '.so' in Path(row['path']).name]
    library_packages = {candidate['package'] for row in libraries for candidate in row['host_candidates']}
    return {
        'schema_version': 1, 'scope': 'All extracted paths plus the AppImage ELF runtime prefix.',
        'captured_at': datetime.now(timezone.utc).isoformat(),
        'inspection_host': {'os_release': Path('/etc/os-release').read_text(encoding='utf-8'),
                            'dpkg_status_sha256': sha256(Path('/var/lib/dpkg/status'))},
        'artifact': {'name': artifact.name, 'bytes': artifact.stat().st_size, 'sha256': sha256(artifact)},
        'runtime': runtime_record(artifact, offset),
        'provenance_limit': 'Host package names/versions describe this inspection host, not a verified build origin. '
                            'Even exact-content matches need file-level license review; package-wide license labels '
                            'can include unshipped build/test sources. Non-ELF files remain hash-inventoried only.',
        'summary': {'paths': len(rows), 'regular_files': sum(row['type'] == 'file' for row in rows),
                    'symlinks': sum(row['type'] == 'symlink' for row in rows),
                    'shared_objects': len(libraries), 'shared_object_package_candidates': len(library_packages),
                    'shared_objects_without_package_candidate': sum(not row['host_candidates'] for row in libraries),
                    'candidate_packages_mentioning_copyleft': sum(item['copyleft_mentioned'] for item in notices),
                    'candidate_packages_without_license_labels': sum(not item['labels'] for item in notices),
                    'candidate_packages_with_shipped_copyright': sum(bool(item['shipped_copyright']) for item in notices)},
        'release_eligible': False,
        'release_blockers': ['Diagnostic inventory does not verify a maintained runtime build recipe or '
                             'per-file licenses. Use check_release_bundle.py for an artifact-specific decision.'],
        'packages': notices, 'files': rows,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--payload', required=True, type=Path)
    parser.add_argument('--appimage', required=True, type=Path)
    parser.add_argument('--runtime-bytes', required=True, type=int)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    try:
        report = inventory(args.appimage.resolve(strict=True), args.payload.resolve(strict=True), args.runtime_bytes)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n', encoding='utf-8')
        print('Diagnostic inventory complete: ' + json.dumps(report['summary'], sort_keys=True))
        print('AppImage release eligible: false; inventory is evidence, not approval.')
        return 0
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f'FAIL: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
