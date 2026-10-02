"""Release decision for audited native AppImage artifacts.

This is deliberately separate from diagnostic payload inventory. Source-package
notice coverage cannot approve a native payload or its statically linked runtime.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess


APPIMAGE_RELEASE_BLOCKERS = (
    "Tauri's unreviewed prebuilt runtime bundles LGPL libfuse, forbidden by the shipped-dependency policy",
    "artifact-specific evidence for the reviewed host-library recipe is required",
)


def _approved_recipe():
    # Importing constants performs no downloads, compilation or package install.
    try:
        import build_thin_appimage
    except ImportError as error:
        raise ValueError('AppImage audited thin builder is unavailable') from error
    return build_thin_appimage


def _sha256(path: Path) -> str:
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def require_releasable_appimage(payload: Path | None = None,
                               artifact: Path | None = None) -> dict:
    """Accept only the reviewed host-library recipe, bound to this actual image.

    There is intentionally no environment-variable override or self-attested
    approval flag. This checks the maintained build recipe's evidence; it is not
    a general binary decompiler or proof against deliberately forged build logs.
    """
    if payload is None or artifact is None:
        raise ValueError(
            "AppImage release is blocked: " + "; ".join(APPIMAGE_RELEASE_BLOCKERS)
            + ". Use the deb with system GTK/WebKit or the audited thin builder; "
            "see docs/licensing.md section 5.1. Diagnostic inspection: scripts/appimage_inventory.py."
        )
    root = payload.resolve(strict=True)
    evidence = root / 'usr/lib/FontCarver/licenses/thin-appimage'
    receipt_path = evidence / 'BUILD.json'
    if not receipt_path.is_file():
        raise ValueError('AppImage release is blocked: missing audited thin-runtime BUILD.json')
    # Validate boundaries before reading any receipt, source or other file.
    elf_paths = []
    for path in sorted(root.rglob('*')):
        relative = path.relative_to(root).as_posix()
        if re.search(r'\.so(?:\.|$)', path.name):
            raise ValueError(f'AppImage bundles a shared library: {relative}')
        if path.is_symlink():
            try:
                resolved = path.resolve(strict=True)
            except (OSError, RuntimeError) as error:
                raise ValueError(f'AppImage has unsafe or broken symlink: {path.relative_to(root)}') from error
            if not resolved.is_relative_to(root):
                raise ValueError(f'AppImage symlink escapes payload: {path.relative_to(root)}')
            continue
        if path.is_dir():
            continue
        if not path.is_file():
            raise ValueError(f'AppImage has unsupported file type: {path.relative_to(root)}')
        if path.stat().st_mode & 0o111 and relative not in ('AppRun', 'usr/bin/fontcarver-desktop'):
            raise ValueError(f'AppImage includes an unapproved executable: {relative}')
        with path.open('rb') as stream:
            if stream.read(4) == b'\x7fELF':
                elf_paths.append(path.relative_to(root).as_posix())
    if elf_paths != ['usr/bin/fontcarver-desktop']:
        raise ValueError('AppImage must contain only the FontCarver ELF executable in its payload')
    recipe = _approved_recipe()
    receipt = json.loads(receipt_path.read_text(encoding='utf-8'))
    if not isinstance(receipt, dict) or receipt.get('schema_version') != 1 or receipt.get('profile') != recipe.PROFILE:
        raise ValueError('AppImage thin-runtime receipt has an unapproved profile/schema')
    if receipt.get('source_commit') != recipe.COMMIT:
        raise ValueError('AppImage runtime receipt source commit is unapproved')
    inputs = receipt.get('inputs')
    if not isinstance(inputs, dict) or set(inputs) != set(recipe.APPROVED_INPUTS) | {'runtime.map'}:
        raise ValueError('AppImage runtime input hash inventory is incomplete')
    for name, expected in recipe.APPROVED_INPUTS.items():
        path = evidence / name
        if inputs.get(name) != expected or not path.is_file() or _sha256(path) != expected:
            raise ValueError(f'AppImage runtime source/license pin mismatch: {name}')
    launcher = root / 'AppRun'
    if not launcher.is_file() or launcher.read_bytes() != recipe.APP_RUN or not os.access(launcher, os.X_OK):
        raise ValueError('AppImage launcher differs from the approved host-library launcher')
    if receipt.get('apprun_sha256') != _sha256(launcher):
        raise ValueError('AppImage launcher receipt hash mismatch')
    runtime = receipt.get('runtime', {})
    if not isinstance(runtime, dict):
        raise ValueError('AppImage runtime receipt is malformed')
    offset = runtime.get('bytes')
    if not isinstance(offset, int) or isinstance(offset, bool) or not 20 <= offset <= 64 * 1024 * 1024:
        raise ValueError('AppImage runtime receipt has invalid offset')
    with artifact.open('rb') as stream:
        prefix = stream.read(offset)
        if len(prefix) != offset or stream.read(4) != b'hsqs':
            raise ValueError('AppImage runtime receipt offset does not match SquashFS')
    if prefix[:6] != b'\x7fELF\x02\x01' or prefix[18:20] != b'\x3e\x00':
        raise ValueError('AppImage approved runtime requires Linux x86_64 ELF')
    digest = hashlib.sha256(prefix).hexdigest()
    if digest != runtime.get('sha256'):
        raise ValueError('AppImage runtime prefix hash mismatch')
    try:
        result = subprocess.run(['readelf', '--dynamic', str(artifact)], capture_output=True,
                                text=True, check=True, timeout=30)
    except (OSError, subprocess.SubprocessError) as error:
        raise ValueError(f'AppImage runtime ELF inspection failed: {error}') from error
    needed = set(re.findall(r'\(NEEDED\).*Shared library: \[([^\]]+)\]', result.stdout))
    if '(RPATH)' in result.stdout or '(RUNPATH)' in result.stdout:
        raise ValueError('AppImage runtime overrides host library search paths')
    claimed = runtime.get('needed')
    if (not isinstance(claimed, list) or not all(isinstance(name, str) for name in claimed)
            or needed != recipe.REQUIRED_NEEDED or set(claimed) != needed):
        raise ValueError('AppImage runtime does not use the approved host shared libraries')
    link_map = evidence / 'runtime.map'
    if not link_map.is_file():
        raise ValueError('AppImage runtime link map is missing')
    if _sha256(link_map) != inputs.get('runtime.map'):
        raise ValueError('AppImage runtime link map hash mismatch')
    map_text = link_map.read_text(encoding='utf-8')
    archives = {Path(name).name for name in re.findall(r'[^\s()]+\.a(?=[\s(]|$)', map_text)}
    # The verified GNU link map lists these compiler archives as LOAD search
    # inputs but extracts no members. Any linked archive member needs review.
    if (archives - {'libgcc.a', 'libc_nonshared.a'}
            or re.search(r'\.a\s*\(', map_text)):
        raise ValueError('AppImage runtime link map contains a forbidden static library')
    if not all(name in map_text for name in ('libfuse3.so', 'libsquashfuse.so', 'libsquashfuse_ll.so')):
        raise ValueError('AppImage runtime link map does not record host-library linkage')
    return {'profile': recipe.PROFILE, 'runtime_sha256': digest,
            'runtime_needed': sorted(needed), 'bundled_shared_libraries': 0,
            'source_and_license_pins': len(recipe.APPROVED_INPUTS),
            'scope': 'Reviewed build recipe and artifact evidence; host libraries are not redistributed.'}
