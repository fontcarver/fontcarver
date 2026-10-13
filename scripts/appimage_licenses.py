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

# A Debian `License:` line is free text. "GPLv2", "GPL2" and the spelled-out name
# all occur, so these deliberately do not require a word boundary after the
# acronym: `\bgpl\b` would classify "GPLv2" as permissive. Order matters, because
# every GPL acronym is a substring of its Affero and Lesser siblings.
_FORBIDDEN = re.compile(r'\b(?:agpl|sspl|affero\s+general\s+public)', re.I)
_LGPL = re.compile(r'\blgpl|\blesser\s+general\s+public', re.I)
_GPL = re.compile(r'\bgpl|\bgeneral\s+public\s+license', re.I)

# Debian writes `and` when different parts of a package fall under each license,
# and `or` when the recipient may choose one. A conjunctive label therefore binds
# by its most restrictive term; a purely disjunctive one may take its most
# permissive branch, which is the same rule docs/licensing.md section 5 applies to
# dual-licensed packages.
_TERMS = re.compile(r'\s+(?:and|or)\s+', re.I)
_CONJUNCTIVE = re.compile(r'\s+and\s+', re.I)
_DISJUNCTIVE = re.compile(r'\s+or\s+', re.I)

# Most restrictive first; _rank is the index, so a smaller rank wins a conflict.
_ORDER = (KIND_FORBIDDEN, KIND_GPL, KIND_LGPL, KIND_OTHER)

# Recorded rulings for GPL-labelled packages whose shipped files are not GPL.
# Each entry needs a reason and a reviewer; a new package version needs a new
# ruling. This is not an extensible allow-list to be appended to casually.
# Determinations recorded against the hosted-runner census of 2026-10-13, each
# made by reading the package's own copyright file. A ruling says why a GPL
# label or an unreadable file does not describe the object this image ships. It
# is valid for a build host's package set, so re-read the census per release.
RULINGS: dict[str, str] = {
    'libatk-bridge2.0-0t64':
        'the GPL appears only as one side of a choice; the other side is taken (see docs/appimage-license-determinations.md)',
    'libatk1.0-0t64':
        'the GPL appears only as one side of a choice; the other side is taken (see docs/appimage-license-determinations.md)',
    'libatomic1':
        'GPL-3 with the GCC Runtime Library Exception 3.1, which exists to permit exactly this (see docs/appimage-license-determinations.md)',
    'libatspi2.0-0t64':
        'the GPL appears only as one side of a choice; the other side is taken (see docs/appimage-license-determinations.md)',
    'libavahi-client3':
        'LGPL stated in prose, no License field to parse; shipped under the section 6 notice (see docs/appimage-license-determinations.md)',
    'libavahi-common3':
        'LGPL stated in prose, no License field to parse; shipped under the section 6 notice (see docs/appimage-license-determinations.md)',
    'libblkid1':
        'GPL covers command-line utilities in the same source package; the shipped library subtree is LGPL (see docs/appimage-license-determinations.md)',
    'libbz2-1.0':
        'GPL covers debian/* packaging only, which no AppImage payload contains (see docs/appimage-license-determinations.md)',
    'libcairo-gobject2':
        'LGPL-2.1 or MPL-1.1 at the recipient choice, stated in prose; shipped under the section 6 notice (see docs/appimage-license-determinations.md)',
    'libcairo2':
        'LGPL-2.1 or MPL-1.1 at the recipient choice, stated in prose; shipped under the section 6 notice (see docs/appimage-license-determinations.md)',
    'libcap2':
        'the GPL appears only as one side of a choice; the other side is taken (see docs/appimage-license-determinations.md)',
    'libcolord2':
        'GPL covers command-line utilities in the same source package; the shipped library subtree is LGPL (see docs/appimage-license-determinations.md)',
    'libdatrie1':
        'GPL covers debian/* packaging only, which no AppImage payload contains (see docs/appimage-license-determinations.md)',
    'libdbus-1-3':
        'GPL covers debian/* packaging only, which no AppImage payload contains (see docs/appimage-license-determinations.md)',
    'libdw1t64':
        'the GPL appears only as one side of a choice; the other side is taken (see docs/appimage-license-determinations.md)',
    'libelf1t64':
        'the GPL appears only as one side of a choice; the other side is taken (see docs/appimage-license-determinations.md)',
    'libenchant-2-2':
        'GPL covers a macOS-only source file that is not built on Linux (see docs/appimage-license-determinations.md)',
    'libevdev2':
        'GPL covers Linux UAPI headers carrying the syscall-note exception (see docs/appimage-license-determinations.md)',
    'libffi8':
        'GPL covers build scaffolding under its own Autoconf/Libtool exception, none of it compiled in (see docs/appimage-license-determinations.md)',
    'libgdk-pixbuf-2.0-0':
        'GPL covers tests, docs or code generators that are not compiled into the shipped object (see docs/appimage-license-determinations.md)',
    'libgdk-pixbuf-2.0-dev':
        'development package matched through an unversioned .so symlink; GPL covers its tests, and a runtime payload should not carry it at all (see docs/appimage-license-determinations.md)',
    'libglib2.0-0t64':
        'GPL covers tests, docs or code generators that are not compiled into the shipped object (see docs/appimage-license-determinations.md)',
    'libglib2.0-dev':
        'development package matched through an unversioned .so symlink; GPL covers its tests, and a runtime payload should not carry it at all (see docs/appimage-license-determinations.md)',
    'libgnutls30t64':
        'GPL covers command-line utilities in the same source package; the shipped library subtree is LGPL (see docs/appimage-license-determinations.md)',
    'libgraphite2-3':
        'GPL covers debian/* packaging only, which no AppImage payload contains (see docs/appimage-license-determinations.md)',
    'libgssapi-krb5-2':
        'MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libgstreamer-gl1.0-0':
        'GPL covers debian/* packaging only, which no AppImage payload contains (see docs/appimage-license-determinations.md)',
    'libgstreamer-plugins-base1.0-0':
        'GPL covers debian/* packaging only, which no AppImage payload contains (see docs/appimage-license-determinations.md)',
    'libgstreamer1.0-0':
        'GPL covers Bison-generated parsers under the Bison exception, which permits non-GPL use (see docs/appimage-license-determinations.md)',
    'libgtk-3-0t64':
        'GPL-3+ covers the build scripts and documentation; the shipped library is LGPL-2+ (see docs/appimage-license-determinations.md)',
    'libharfbuzz-icu0':
        'GPL covers build scaffolding under its own Autoconf/Libtool exception, none of it compiled in (see docs/appimage-license-determinations.md)',
    'libhogweed6t64':
        'GPL covers build scaffolding under its own Autoconf/Libtool exception, none of it compiled in (see docs/appimage-license-determinations.md)',
    'libhyphen0':
        'GPL covers debian/* packaging only, which no AppImage payload contains (see docs/appimage-license-determinations.md)',
    'libicu74':
        'GPL covers command-line utilities in the same source package; the shipped library subtree is LGPL (see docs/appimage-license-determinations.md)',
    'libidn2-0':
        'the GPL appears only as one side of a choice; the other side is taken (see docs/appimage-license-determinations.md)',
    'libjavascriptcoregtk-4.1-0':
        'GPL covers Bison-generated parsers under the Bison exception, which permits non-GPL use (see docs/appimage-license-determinations.md)',
    'libk5crypto3':
        'MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libkeyutils1':
        'GPL covers command-line utilities in the same source package; the shipped library subtree is LGPL (see docs/appimage-license-determinations.md)',
    'libkrb5-3':
        'MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libkrb5support0':
        'MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'liblcms2-2':
        'GPL covers debian/* packaging only, which no AppImage payload contains (see docs/appimage-license-determinations.md)',
    'liblz4-1':
        'GPL covers tests, docs or code generators that are not compiled into the shipped object (see docs/appimage-license-determinations.md)',
    'liblzma5':
        'GPL covers tests, docs or code generators that are not compiled into the shipped object (see docs/appimage-license-determinations.md)',
    'libmount1':
        'GPL covers command-line utilities in the same source package; the shipped library subtree is LGPL (see docs/appimage-license-determinations.md)',
    'libnettle8t64':
        'GPL covers build scaffolding under its own Autoconf/Libtool exception, none of it compiled in (see docs/appimage-license-determinations.md)',
    'libnghttp2-14':
        'GPL covers build scaffolding under its own Autoconf/Libtool exception, none of it compiled in (see docs/appimage-license-determinations.md)',
    'liborc-0.4-0t64':
        '2-clause BSD stated in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libp11-kit0':
        'GPL covers build scaffolding under its own Autoconf/Libtool exception, none of it compiled in (see docs/appimage-license-determinations.md)',
    'libpixman-1-0':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libpng16-16t64':
        'the GPL appears only as one side of a choice; the other side is taken (see docs/appimage-license-determinations.md)',
    'libsecret-1-0':
        'GPL covers tests, docs or code generators that are not compiled into the shipped object (see docs/appimage-license-determinations.md)',
    'libselinux1':
        'GPL covers debian/* packaging only, which no AppImage payload contains (see docs/appimage-license-determinations.md)',
    'libsqlite3-0':
        'GPL covers debian/* packaging only, which no AppImage payload contains (see docs/appimage-license-determinations.md)',
    'libsystemd0':
        'GPL covers Linux UAPI headers carrying the syscall-note exception (see docs/appimage-license-determinations.md)',
    'libtasn1-6':
        'LGPL stated in prose, no License field to parse; shipped under the section 6 notice (see docs/appimage-license-determinations.md)',
    'libthai0':
        'GPL covers debian/* packaging only, which no AppImage payload contains (see docs/appimage-license-determinations.md)',
    'libudev1':
        'GPL covers Linux UAPI headers carrying the syscall-note exception (see docs/appimage-license-determinations.md)',
    'libunistring5':
        'GPL covers build scaffolding under its own Autoconf/Libtool exception, none of it compiled in (see docs/appimage-license-determinations.md)',
    'libunwind8':
        'GPL covers debian/* packaging only, which no AppImage payload contains (see docs/appimage-license-determinations.md)',
    'libwebkit2gtk-4.1-0':
        'GPL covers Bison-generated parsers under the Bison exception, which permits non-GPL use (see docs/appimage-license-determinations.md)',
    'libxau6':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxcb-render0':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxcb-shm0':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxcomposite1':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxcursor1':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxdamage1':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxdmcp6':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxext6':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxfixes3':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxi6':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxinerama1':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxkbcommon0':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxrandr2':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxrender1':
        'X Consortium / Open Group MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libxslt1.1':
        'MIT permission grant in prose, no License field to parse (see docs/appimage-license-determinations.md)',
    'libzstd1':
        'the GPL appears only as one side of a choice; the other side is taken (see docs/appimage-license-determinations.md)',
}


def _classify_term(term: str) -> str:
    """Classify one license name, ignoring any trailing exception wording."""
    if _FORBIDDEN.search(term):
        return KIND_FORBIDDEN
    if _LGPL.search(term):
        return KIND_LGPL
    if _GPL.search(term):
        return KIND_GPL
    return KIND_OTHER


def _rank(kind: str) -> int:
    return _ORDER.index(kind)


def classify_label(label: str) -> str:
    """Classify one Debian `License:` label, which may combine several names."""
    kinds = {_classify_term(term) for term in _TERMS.split(label) if term.strip()}
    if not kinds:
        return KIND_OTHER
    if _DISJUNCTIVE.search(label) and not _CONJUNCTIVE.search(label):
        return max(kinds, key=_rank)
    return min(kinds, key=_rank)


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
    """Apply recorded rulings on top of a label classification.

    A ruling answers both an unreadable copyright file and a GPL label whose
    stanzas cover files the payload does not contain; most pre-DEP-5 copyright
    files state permissive terms in prose a classifier cannot parse. It never
    answers a forbidden license: that is a policy question, not a reading.
    """
    result = dict(classify_labels(labels), package=name, permitted=False, reason=None)
    if result['verdict'] == 'pass':
        result['permitted'] = True
    elif result['verdict'] in ('ruling-required', 'unidentified'):
        # A ruling is about a package, not about one architecture's build of it.
        reason = RULINGS.get(name) or RULINGS.get(name.split(':', 1)[0])
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
