#!/usr/bin/env python3
"""Classify licenses of libraries a native bundler copied into an artifact.

Debian labels describe source packages, so a GPL label can belong to utilities
that were never shipped. This module therefore never auto-passes a GPL label; it
demands a recorded ruling. It makes no network or subprocess calls.
"""
from __future__ import annotations

import re

KIND_LGPL = 'lgpl'
KIND_GPL = 'gpl'
KIND_FORBIDDEN = 'forbidden'
KIND_OTHER = 'other'

# Order matters: LGPL is matched before GPL because 'GPL' is its substring.
_LGPL = re.compile(r'^\s*lgpl\b', re.I)
_FORBIDDEN = re.compile(r'\b(?:agpl|sspl)\b', re.I)
_GPL = re.compile(r'\bgpl\b|\bgpl-\d', re.I)

# Recorded rulings for GPL-labelled packages whose shipped files are not GPL.
# Each entry needs a reason and a reviewer; a new package version needs a new
# ruling. This is not an extensible allow-list to be appended to casually.
RULINGS: dict[str, str] = {}


def classify_label(label: str) -> str:
    """Classify one Debian `License:` label."""
    if _LGPL.match(label):
        return KIND_LGPL
    if _FORBIDDEN.search(label):
        return KIND_FORBIDDEN
    if _GPL.search(label):
        return KIND_GPL
    return KIND_OTHER


def classify_labels(labels: list[str]) -> dict:
    """Decide a package's verdict from every label its copyright file carries."""
    if not labels:
        return {'verdict': 'unidentified', 'kinds': [], 'requires_lgpl_notice': False}
    kinds = sorted({classify_label(label) for label in labels})
    notice = KIND_LGPL in kinds
    if KIND_FORBIDDEN in kinds:
        verdict = 'forbidden'
    elif KIND_GPL in kinds:
        verdict = 'ruling-required'
    else:
        verdict = 'pass'
    return {'verdict': verdict, 'kinds': kinds, 'requires_lgpl_notice': notice}


SOURCE_BASE = 'https://sources.debian.org/src'
SOURCE_UNAVAILABLE = 'UNAVAILABLE (see SOURCE-OFFER.txt)'


def source_reference(source_package: str, source_version: str) -> str:
    """Point at the source for one exact package version, for LGPL-2.1 section 6."""
    if not source_package or not source_version:
        raise ValueError('source reference needs a package and a version')
    return f'{SOURCE_BASE}/{source_package}/{source_version}/'


def verdict_for_package(name: str, labels: list[str]) -> dict:
    """Apply recorded rulings on top of a label classification."""
    result = dict(classify_labels(labels), package=name, permitted=False, reason=None)
    if result['verdict'] == 'pass':
        result['permitted'] = True
    elif result['verdict'] == 'ruling-required':
        reason = RULINGS.get(name)
        if reason:
            result.update(permitted=True, reason=reason)
    return result


def render_bundled_libraries(entries: list[dict]) -> str:
    """Render the per-library notice that discharges the naming obligation.

    A library with no resolvable source is still listed by name; blocking the
    release is the gate's job, not this renderer's.
    """
    if not entries:
        raise ValueError('cannot render a notice with no bundled libraries')
    lines = [
        'Bundled shared libraries in this AppImage',
        '',
        'This image carries copies of system libraries. They remain under their own',
        "licenses; including them does not change FontCarver's MIT OR Apache-2.0 terms.",
        'Libraries under the LGPL may be replaced: extract the image, substitute an',
        'interface-compatible library, and repack. The LGPL-2 and LGPL-2.1 text',
        'accompanies this notice.',
        '',
        'Package names and versions describe the build host and are not a verified',
        'build provenance record. Source links address LGPL-2.1 section 6.',
        '',
    ]
    for entry in sorted(entries, key=lambda item: item['path']):
        if entry['source_package'] and entry['source_version']:
            source = source_reference(entry['source_package'], entry['source_version'])
        else:
            source = SOURCE_UNAVAILABLE
        package = f'{entry["package"]} {entry["version"]}'.strip() or 'no owning package'
        lines.append(f'{entry["path"]}')
        lines.append(f'    package: {package}')
        lines.append(f'    license: {", ".join(entry["labels"]) or "UNIDENTIFIED"}')
        lines.append(f'    source:  {source}')
    lines.append('')
    lines.append('Full license texts accompany this notice: LGPL-2 text, LGPL-2.1 text.')
    return '\n'.join(lines) + '\n'
