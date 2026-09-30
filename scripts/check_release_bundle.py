#!/usr/bin/env python3
"""Extract native installers, inspect payloads, and record SHA-256 evidence.

This checks package contents without launching FontCarver or installing it.
Native tools are required; a missing tool/platform fails verification.
"""
from __future__ import annotations

import argparse
import configparser
import hashlib
import json
import os
import plistlib
import re
import shlex
import subprocess
import sys
import tempfile
from contextlib import contextmanager
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
RESOURCES = ('LICENSE-MIT', 'LICENSE-APACHE', 'THIRD_PARTY.md',
             'DEPENDENCIES.json', 'INSTALL.md')
HOSTS = {'deb': 'linux', 'appimage': 'linux', 'dmg': 'darwin', 'msi': 'win32'}


class VerificationError(Exception):
    """An installer or its extracted payload failed validation."""


def sha256(path: Path) -> str:
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def run(command: list[str], cwd: Path | None = None) -> bytes:
    try:
        result = subprocess.run(command, cwd=cwd, capture_output=True, timeout=300)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise VerificationError(f'{command[0]} could not complete: {error}') from error
    if result.returncode:
        detail = (result.stderr or result.stdout).decode(errors='replace')[-3000:]
        raise VerificationError(f'{command[0]} exited {result.returncode}: {detail}')
    return result.stdout


def discover(bundle: Path, expected: list[str]) -> list[tuple[str, Path]]:
    artifacts = []
    for kind in expected:
        if kind not in HOSTS:
            raise VerificationError(f'unsupported artifact type: {kind}')
        matches = sorted(path for path in bundle.rglob('*')
                         if path.is_file() and path.suffix.lower() == f'.{kind}')
        if not matches:
            raise VerificationError(f'missing expected {kind} installer in {bundle}')
        for path in matches:
            if path.is_symlink() or path.stat().st_size == 0:
                raise VerificationError(f'installer is empty or a symlink: {path}')
            artifacts.append((kind, path))
    return artifacts


@contextmanager
def extracted(artifact: Path, kind: str, scratch: Path):
    """Yield (payload root, native metadata); always detach a mounted DMG."""
    if sys.platform != HOSTS[kind]:
        raise VerificationError(f'{kind} verification requires {HOSTS[kind]}, got {sys.platform}')
    payload = scratch / 'payload'
    payload.mkdir()
    artifact = artifact.resolve()
    if kind == 'deb':
        fields = run(['dpkg-deb', '--field', str(artifact), 'Package', 'Version',
                      'Architecture']).decode()
        metadata = dict(line.split(': ', 1) for line in fields.splitlines() if ': ' in line)
        if not all(metadata.get(key) for key in ('Package', 'Version', 'Architecture')):
            raise VerificationError('deb lacks Package, Version, or Architecture metadata')
        run(['dpkg-deb', '--extract', str(artifact), str(payload)])
        yield payload, metadata
    elif kind == 'appimage':
        if not os.access(artifact, os.X_OK):
            raise VerificationError(f'AppImage lacks executable permission: {artifact}')
        run([str(artifact), '--appimage-extract'], cwd=payload)
        yield payload / 'squashfs-root', {'extraction': '--appimage-extract'}
    elif kind == 'dmg':
        attached = False
        try:
            output = run(['hdiutil', 'attach', '-readonly', '-nobrowse', '-plist',
                          '-mountpoint', str(payload), str(artifact)])
            attached = True
            details = plistlib.loads(output)
            yield payload, {'mount_readonly': True, 'entities': len(details['system-entities'])}
        finally:
            if attached:
                run(['hdiutil', 'detach', str(payload)])
    else:
        run(['msiexec.exe', '/a', str(artifact), '/qn', f'TARGETDIR={payload}',
             '/L*V', str(scratch / 'msi-extract.log')])
        yield payload, {'extraction': 'msiexec administrative extraction'}


def inside_file(path: Path, root: Path) -> bool:
    if not path.resolve().is_relative_to(root.resolve()):
        raise VerificationError(f'payload file points outside its root: {path}')
    return path.is_file() and path.stat().st_size > 0


def forbidden_path(path: Path, root: Path) -> bool:
    for part in path.relative_to(root).parts:
        name = part.lower()
        if name in ('fontforge', 'libfontforge') or re.match(r'^libfontforge[._-]', name):
            return True
        if re.match(r'^fontforge(?:[-_.].*)?\.(?:exe|dll|so|dylib|c|h|py|zip|tar(?:\..+)?|tgz)$', name):
            return True
        if re.match(r'^fontforge[-_](?:\d|master|main|source|src)', name):
            return True
    return False


def check_notices(manifest: Path) -> dict:
    try:
        data = json.loads(manifest.read_text(encoding='utf-8'))
    except (ValueError, UnicodeError) as error:
        raise VerificationError(f'invalid DEPENDENCIES.json: {error}') from error
    if not isinstance(data, dict) or data.get('schema_version') != 1:
        raise VerificationError('DEPENDENCIES.json requires schema_version 1')
    packages = data.get('packages')
    if not isinstance(packages, list) or not packages:
        raise VerificationError('DEPENDENCIES.json has no package records')
    count = 0
    for package in packages:
        if not isinstance(package, dict) or not all(package.get(key) for key in
                                                   ('ecosystem', 'name', 'version', 'license')):
            raise VerificationError('dependency package lacks identity or license')
        notices = package.get('notices')
        if not isinstance(notices, list) or not notices:
            raise VerificationError(f'dependency lacks notices: {package["name"]}')
        for notice in notices:
            if not isinstance(notice, dict) or not isinstance(notice.get('path'), str):
                raise VerificationError('invalid dependency notice path')
            relative = PurePosixPath(notice['path'])
            if relative.is_absolute() or '..' in relative.parts or '\\' in notice['path'] or ':' in notice['path']:
                raise VerificationError(f'unsafe dependency notice path: {relative}')
            path = manifest.parent.joinpath(*relative.parts)
            if not inside_file(path, manifest.parent):
                raise VerificationError(f'missing dependency notice: {relative}')
            if sha256(path) != notice.get('sha256'):
                raise VerificationError(f'dependency notice checksum mismatch: {relative}')
            count += 1
    return {'packages': len(packages), 'license_files': count, 'checksums_verified': True}


def platform_metadata(root: Path, files: list[Path], kind: str, binaries: list[Path]) -> dict:
    if kind in ('deb', 'appimage'):
        if kind == 'appimage' and (not inside_file(root / 'AppRun', root)
                                   or not os.access(root / 'AppRun', os.X_OK)):
            raise VerificationError('AppImage lacks executable AppRun launcher')
        desktops = [path for path in files if path.suffix == '.desktop'
                    and path.parent in (root, root / 'usr/share/applications')]
        for desktop in desktops:
            if not inside_file(desktop, root):
                continue
            config = configparser.ConfigParser(interpolation=None)
            try:
                config.read_string(desktop.read_text(encoding='utf-8'))
                entry = config['Desktop Entry']
                executable = shlex.split(entry.get('Exec', ''))
                valid = (entry.get('Type') == 'Application' and entry.get('Name')
                         and executable and Path(executable[0]).name == 'fontcarver-desktop')
            except (ValueError, KeyError, configparser.Error):
                continue
            if valid:
                icon_name = Path(entry.get('Icon', '')).stem
                icons = [path for path in files if path.suffix.lower() in ('.png', '.svg', '.xpm')
                         and path.stem == icon_name and inside_file(path, root)]
                if icons:
                    return {'desktop': str(desktop.relative_to(root)),
                            'icons': [str(path.relative_to(root)) for path in icons]}
        raise VerificationError('missing valid desktop application entry or matching icon')
    if kind == 'dmg':
        for info in files:
            if info.name != 'Info.plist' or info.parent.name != 'Contents':
                continue
            if not inside_file(info, root):
                continue
            data = plistlib.loads(info.read_bytes())
            if data.get('CFBundleExecutable') != 'fontcarver-desktop' or not data.get('CFBundleIdentifier'):
                continue
            if info.parent / 'MacOS/fontcarver-desktop' not in binaries:
                continue
            icons = list((info.parent / 'Resources').glob('*.icns'))
            if any(inside_file(icon, root) for icon in icons):
                return {'bundle_identifier': data['CFBundleIdentifier'],
                        'info_plist': str(info.relative_to(root))}
        raise VerificationError('missing application Info.plist or bundled icns icon')
    return {'executable_format': 'PE', 'icons': 'embedded in executable / installer'}


def inspect_payload(root: Path, kind: str) -> dict:
    paths = sorted(root.rglob('*'))
    for path in paths:
        if forbidden_path(path, root):
            raise VerificationError(f'FontForge binary or sources in payload: {path.relative_to(root)}')
    files = [path for path in paths if path.is_file() or path.is_symlink()]
    executable_name = 'fontcarver-desktop.exe' if kind == 'msi' else 'fontcarver-desktop'
    binaries = [path for path in files if path.name == executable_name and inside_file(path, root)]
    if kind in ('deb', 'appimage'):
        binaries = [path for path in binaries if path == root / 'usr/bin/fontcarver-desktop']
    elif kind == 'dmg':
        binaries = [path for path in binaries if path.parent.name == 'MacOS'
                    and path.parent.parent.name == 'Contents'
                    and path.parent.parent.parent.suffix == '.app']
    signatures = {'deb': (b'\x7fELF',), 'appimage': (b'\x7fELF',), 'msi': (b'MZ',),
                  'dmg': (b'\xcf\xfa\xed\xfe', b'\xce\xfa\xed\xfe', b'\xfe\xed\xfa\xcf',
                          b'\xfe\xed\xfa\xce', b'\xca\xfe\xba\xbe', b'\xbe\xba\xfe\xca')}
    valid_binaries = []
    for path in binaries:
        with path.open('rb') as stream:
            header = stream.read(4)
        if any(header.startswith(signature) for signature in signatures[kind]) and (
                kind == 'msi' or os.access(path, os.X_OK)):
            valid_binaries.append(path)
    if not valid_binaries:
        raise VerificationError(f'missing valid application executable: {executable_name}')
    resource_roots = [path.parent for path in files if path.name == 'DEPENDENCIES.json']
    resource_root = None
    instructions = None
    for candidate in resource_roots:
        if all(inside_file(candidate / name, root) for name in RESOURCES[:-1]):
            for ancestor in (candidate, *candidate.parents):
                if not ancestor.is_relative_to(root):
                    break
                if inside_file(ancestor / 'INSTALL.md', root):
                    instructions = ancestor / 'INSTALL.md'
                    break
            resource_root = candidate
            break
    if resource_root is None or instructions is None:
        raise VerificationError('missing or empty release resources: ' + ', '.join(RESOURCES))
    resource_paths = [resource_root / name for name in RESOURCES[:-1]] + [instructions]
    resources = [{'path': str(path.relative_to(root)), 'sha256': sha256(path)}
                 for path in resource_paths]
    return {'executable': {'paths': [str(path.relative_to(root)) for path in valid_binaries],
                           'valid_header': True},
            'resources': resources, 'notices': check_notices(resource_root / 'DEPENDENCIES.json'),
            'platform_metadata': platform_metadata(root, files, kind, valid_binaries),
            'fontforge_paths_absent': True, 'file_count': len(files)}


def verify(bundle: Path, expected: list[str]) -> list[dict]:
    artifacts = discover(bundle, expected)
    scratch = ROOT / '.scratch' / 'p5-release-verify'
    scratch.mkdir(parents=True, exist_ok=True)
    rows = []
    try:
        for kind, artifact in artifacts:
            with tempfile.TemporaryDirectory(prefix=f'{kind}-', dir=scratch) as directory:
                with extracted(artifact, kind, Path(directory)) as (root, metadata):
                    payload = inspect_payload(root, kind)
                rows.append({'path': str(artifact.relative_to(bundle)), 'type': kind,
                             'bytes': artifact.stat().st_size, 'sha256': sha256(artifact),
                             'metadata': metadata, 'payload': payload})
    finally:
        try:
            scratch.rmdir()
        except OSError:
            pass
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bundle-dir', type=Path, default=ROOT / 'target/release/bundle')
    parser.add_argument('--expected', required=True, help='comma-separated deb,appimage,dmg,msi')
    parser.add_argument('--output', type=Path, help='default: BUNDLE_DIR/release-manifest.json')
    args = parser.parse_args()
    expected = list(dict.fromkeys(part.strip().lower() for part in args.expected.split(',')))
    report = {'schema_version': 1, 'platform': sys.platform, 'expected': expected,
              'generated_at': datetime.now(timezone.utc).isoformat(), 'ok': False}
    try:
        report['artifacts'] = verify(args.bundle_dir.resolve(), expected)
        report['ok'] = True
    except (VerificationError, OSError, ValueError, KeyError, plistlib.InvalidFileException) as error:
        report['error'] = str(error)
    output = args.output or args.bundle_dir / 'release-manifest.json'
    try:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    except OSError as error:
        print(f'FAIL: cannot write manifest: {error}', file=sys.stderr)
        return 1
    print(f'{"PASS" if report["ok"] else "FAIL"}: {output}')
    if not report['ok']:
        print(report['error'], file=sys.stderr)
    return 0 if report['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
