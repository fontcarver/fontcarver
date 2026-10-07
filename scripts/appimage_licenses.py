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
