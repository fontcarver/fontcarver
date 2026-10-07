#!/usr/bin/env python3
"""Release decision for an AppImage whose bundler copied in system libraries.

LGPL libraries may ship with the notices that LGPL-2.1 section 6 requires. GPL,
AGPL and SSPL may not, and a library whose license cannot be identified blocks
the release until it is. Debian labels describe source packages, so a GPL label
demands a recorded ruling rather than an automatic refusal or an automatic pass.
This module never executes the artifact or any payload binary.
"""
from __future__ import annotations

from pathlib import Path

import appimage_inventory as inv
from appimage_licenses import verdict_for_package

NOTICE_DIR = 'usr/lib/FontCarver/licenses/appimage'
NOTICE_NAME = 'BUNDLED-LIBRARIES.txt'
REQUIRED_TEXTS = ('HOST-LGPL-2.txt', 'HOST-LGPL-2.1.txt')


def resolve_libraries(payload: Path) -> list[dict]:
    """List every bundled shared object with its host package and license labels."""
    rows = inv.payload_files(payload)
    libraries = [row for row in rows
                 if 'elf_machine' in row and '.so' in Path(row['path']).name]
    if not libraries:
        return []
    owners, packages = inv.host_inventory({Path(row['path']).name for row in libraries})
    inv.annotate_host_candidates(rows, owners, packages)
    notices = {entry['package']: entry
               for entry in inv.package_notices(payload, rows, packages)}
    resolved = []
    for row in libraries:
        candidates = row.get('host_candidates', [])
        entry = notices.get(candidates[0]['package']) if candidates else None
        resolved.append({
            'path': row['path'],
            'package': entry['package'] if entry else '',
            'version': entry.get('version', '') if entry else '',
            'source_package': entry.get('source_package', '') if entry else '',
            'source_version': entry.get('source_version', '') if entry else '',
            'labels': entry.get('labels', []) if entry else [],
        })
    return resolved


def require_releasable_appimage(payload: Path | None = None,
                                artifact: Path | None = None) -> dict:
    """Accept only a payload whose every bundled library is permitted and noticed.

    There is intentionally no environment-variable override or self-attested
    approval flag. Host package data is a hint about this inspection host, not a
    verified build provenance record.
    """
    if payload is None or artifact is None:
        raise ValueError('AppImage release needs both a payload and artifact path')
    root = payload.resolve(strict=True)
    libraries = resolve_libraries(root)
    forbidden = []
    for item in libraries:
        verdict = verdict_for_package(item['package'], item['labels'])
        if not verdict['permitted']:
            forbidden.append(f'{item["path"]} ({item["package"] or "no owning package"}: '
                             f'{", ".join(item["labels"]) or "unidentified"})')
    if forbidden:
        raise ValueError('AppImage release is blocked by bundled libraries: '
                         + '; '.join(sorted(forbidden)))
    notice = root / NOTICE_DIR / NOTICE_NAME
    if not notice.is_file():
        raise ValueError(f'AppImage release is blocked: missing {NOTICE_DIR}/{NOTICE_NAME}')
    text = notice.read_text(encoding='utf-8', errors='replace')
    missing = sorted(item['path'] for item in libraries if item['path'] not in text)
    if missing:
        raise ValueError('AppImage notice does not account for: ' + '; '.join(missing))
    for name in REQUIRED_TEXTS:
        if not (root / NOTICE_DIR / name).is_file():
            raise ValueError(f'AppImage release is blocked: missing {NOTICE_DIR}/{name}')
    return {'bundled_shared_libraries': len(libraries), 'forbidden': [],
            'notice': f'{NOTICE_DIR}/{NOTICE_NAME}',
            'scope': 'Bundled-library licenses from host package labels; '
                     'host data is a hint, not verified build provenance.'}
