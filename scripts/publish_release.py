#!/usr/bin/env python3
"""Stage the installers a tag publishes, and write that release's notes.

The release job downloads one artifact per platform, each holding the installers
that platform built and the `release-manifest.json` its own inspection wrote.
This collects them into one flat directory for `gh release upload`, and refuses
to publish unless every platform is present, reported `ok`, was built from the
revision the tag names, and still carries the exact bytes its manifest recorded.

    python3 scripts/publish_release.py --tag v0.1.0-alpha.1 \\
        --source downloads --destination assets --revision "$(git rev-parse HEAD)"

`--check-tag` alone compares a tag against the version in the working tree, so a
mistyped tag fails before a ninety-minute three-platform build, not after it.
Publishing itself stays in the workflow: this writes files and nothing else.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import sys
import tomllib

sys.path.insert(0, str(Path(__file__).resolve().parent))

import build_release

ROOT = Path(__file__).resolve().parent.parent
# Every format the tag is expected to offer, per platform label in release.yml.
# A row missing one of its formats is a partial release, not a smaller one.
EXPECTED = {'linux-x86_64': ('deb', 'rpm', 'appimage'),
            'macos-arm64': ('dmg',),
            'windows-x86_64': ('nsis', 'portable')}
DESCRIPTIONS = {
    'deb': 'Debian and Ubuntu (`apt install ./<file>`)',
    'rpm': 'Fedora and other rpm distributions (`dnf install ./<file>`)',
    'appimage': 'any Linux distribution with glibc at least as new as Ubuntu 24.04',
    'dmg': 'macOS on Apple silicon — there is no Intel build',
    'nsis': 'Windows installer, for all users, English and Chinese',
    'portable': 'Windows without installing: unpack and run, data stays beside the executable',
}


class ReleaseError(Exception):
    """A downloaded platform artifact cannot be published as it stands."""


def tag_version(tag: str) -> str:
    """Return the version a release tag names, rejecting anything else."""
    if not tag.startswith('v'):
        raise ValueError(f'a release tag starts with v: {tag}')
    # Reuse the release build's own rule, so a tag cannot name a version the
    # native installers could not have encoded in the first place.
    return build_release.validate_versions(tag[1:], tag[1:], tag[1:])


def is_prerelease(version: str) -> bool:
    """Report whether a version carries a SemVer pre-release identifier."""
    return '-' in version


def configured_version(root: Path = ROOT) -> str:
    """Return the single version the working tree declares for a release."""
    manifest = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))
    frontend = json.loads((root / 'app/package.json').read_text(encoding='utf-8'))
    tauri = json.loads((root / 'src-tauri/tauri.conf.json').read_text(encoding='utf-8'))
    return build_release.validate_versions(manifest['workspace']['package']['version'],
                                           frontend['version'], tauri['version'])


def sha256(path: Path) -> str:
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read_platform(directory: Path, label: str, version: str, revision: str) -> dict:
    """Validate one downloaded platform artifact and locate its installers."""
    manifest_path = directory / 'target/release/bundle/release-manifest.json'
    if not manifest_path.is_file():
        raise ReleaseError(f'{label}: no release-manifest.json in the uploaded artifact')
    try:
        manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
    except (OSError, ValueError) as error:
        raise ReleaseError(f'{label}: unreadable release-manifest.json: {error}') from error
    if not manifest.get('ok'):
        raise ReleaseError(f'{label}: the build reported a failed inspection: '
                           f'{manifest.get("error", "no reason recorded")}')
    built = (manifest.get('source') or {}).get('revision')
    if built != revision:
        raise ReleaseError(f'{label}: built from revision {built}, but the tag names {revision}')
    rows = manifest.get('artifacts') or []
    found = {}
    for row in rows:
        artifact = manifest_path.parent / Path(row['path'])
        if found.setdefault(row['type'], artifact) is not artifact:
            raise ReleaseError(f'{label}: two {row["type"]} installers were built; '
                               'publish one candidate, not a directory of them')
        if not artifact.is_file():
            raise ReleaseError(f'{label}: {row["path"]} is in the manifest but not the artifact')
        if artifact.stat().st_size != row['bytes'] or sha256(artifact) != row['sha256']:
            raise ReleaseError(f'{label}: {row["path"]} no longer matches the sha256 its '
                               'inspection recorded; do not publish it')
        if version not in artifact.name:
            raise ReleaseError(f'{label}: {artifact.name} does not carry version {version}')
    missing = [kind for kind in EXPECTED[label] if kind not in found]
    if missing:
        raise ReleaseError(f'{label}: the release needs {", ".join(missing)} and the '
                           'artifact does not carry it')
    return {'manifest': manifest_path, 'artifacts': found}


def stage(source: Path, destination: Path, version: str, revision: str) -> dict:
    """Collect every platform's installers and manifest into one upload directory."""
    directories = {}
    for directory in sorted(path for path in source.iterdir() if path.is_dir()):
        for label in EXPECTED:
            if directory.name.startswith(f'FontCarver-{label}-'):
                directories[label] = directory
    absent = [label for label in EXPECTED if label not in directories]
    if absent:
        raise ReleaseError('no uploaded artifact for ' + ', '.join(absent)
                           + f'; {source} holds '
                           + (', '.join(path.name for path in source.iterdir()) or 'nothing'))
    platforms = {label: read_platform(directories[label], label, version, revision)
                 for label in EXPECTED}
    destination.mkdir(parents=True, exist_ok=True)
    published, assets = {}, {}
    for label, platform in platforms.items():
        names = []
        for kind in EXPECTED[label]:
            artifact = platform['artifacts'][kind]
            if artifact.name in assets:
                raise ReleaseError(f'{artifact.name} was built by two platforms; '
                                   'a release asset name has to be unique')
            shutil.copy2(artifact, destination / artifact.name)
            assets[artifact.name] = sha256(destination / artifact.name)
            names.append(artifact.name)
        # The manifest is published as built: its digests are the evidence that
        # the uploaded installer is the one that was inspected.
        name = f'release-manifest-{label}.json'
        shutil.copy2(platform['manifest'], destination / name)
        assets[name] = sha256(destination / name)
        published[label] = names
    sums = destination / 'SHA256SUMS.txt'
    sums.write_text(''.join(f'{digest}  {name}\n'
                            for name, digest in sorted(assets.items())), encoding='utf-8')
    assets[sums.name] = sha256(sums)
    return {'version': version, 'revision': revision, 'platforms': published,
            'prerelease': is_prerelease(version),
            'assets': [{'name': name, 'sha256': digest, 'kind': kind}
                       for name, digest, kind in _described(assets, platforms)]}


def _described(assets: dict[str, str], platforms: dict) -> list[tuple[str, str, str]]:
    """Pair each staged asset with the installer format it came from."""
    kinds = {artifact.name: kind for platform in platforms.values()
             for kind, artifact in platform['artifacts'].items()}
    return [(name, digest, kinds.get(name, 'evidence')) for name, digest in sorted(assets.items())]


def notes(report: dict, run_url: str, repo_url: str = '', tag: str = '') -> str:
    """Write release notes that state what was verified and what was not.

    A release body is not rendered inside the repository tree, so a relative
    document link in it resolves to nothing; every link here is absolute and
    pinned to the tag, which is also the revision the files were built from.
    """
    def document(name: str) -> str:
        target = f'{repo_url}/blob/{tag}/{name}' if repo_url and tag else name
        return f'[{name}]({target})'

    version = report['version']
    kinds = {asset['name']: asset['kind'] for asset in report['assets']}
    lines = [f'FontCarver {version} — native desktop builds for Linux, macOS and Windows.', '',
             '## Downloads', '']
    for label in EXPECTED:
        for name in report['platforms'][label]:
            lines.append(f'- **`{name}`** — {DESCRIPTIONS[kinds[name]]}')
    evidence = [asset['name'] for asset in report['assets'] if asset['kind'] == 'evidence']
    lines += ['',
              'Installation and removal for every format: '
              + document('docs/install.md') + '.',
              '',
              'The rest of the files are evidence rather than downloads: '
              + ', '.join(f'`{name}`' for name in evidence) + '.', '',
              '## These builds are not signed', '',
              'FontCarver has no code-signing certificate yet, so the macOS application is '
              'not signed or notarized and the Windows installer carries no Authenticode '
              'signature. Both systems will stop you:', '',
              '- **macOS** refuses the application on first launch. Open it from the Finder '
              'with Control-click → *Open*, or clear the quarantine flag with '
              '`xattr -d com.apple.quarantine /Applications/FontCarver.app`.',
              '- **Windows** shows a SmartScreen warning. *More info* → *Run anyway*.', '',
              'Do that only because you decided to trust this download. Verify it first with '
              '`SHA256SUMS.txt` below.', '',
              '## What was verified', '',
              f'Every file here was built and checked from revision `{report["revision"]}` '
              f'by [this workflow run]({run_url}), on the platform it targets:', '',
              '- the deb is installed and removed on Ubuntu 24.04, the rpm inside a Fedora '
              'container, and the AppImage is launched headless to prove its bundled runtime '
              'is complete;',
              '- the disk image is mounted, the application copied out and started;',
              '- the Windows installer is installed silently, started and uninstalled, and the '
              'portable build is run from its extracted folder to confirm it keeps its data '
              'beside the executable;',
              '- each package is unpacked and inspected for its executable, desktop metadata, '
              'icons and license notices. `release-manifest-<platform>.json` records that '
              'inspection and the digest of the exact file published here.', '',
              '## What was not verified', '',
              'This is an alpha. The native acceptance checklist in '
              + document('docs/releasing.md')
              + ' — opening a font through the native file '
              'dialog, editing, exporting, and upgrading over a previous install while saved '
              'fonts, preferences and recovery snapshots are kept — has not been completed on '
              'any platform. Nothing here is signed or notarized. Keep backups of fonts you '
              'open with it.', '']
    if is_prerelease(version):
        lines += ['## A note for Linux package managers', '',
                  f'`dpkg` and `rpm` both sort `{version}` *after* `'
                  f'{version.split("-", 1)[0]}`, because a hyphen does not mean pre-release to '
                  'either of them. Installing a later final release of the same patch version '
                  'over this one would therefore be a downgrade, which `apt` asks you to '
                  'confirm and `dnf` refuses without `dnf downgrade`. The first final release '
                  'after this series will use a higher patch version to avoid it.', '']
    lines += ['## Verifying your download', '',
              '```sh', 'sha256sum -c SHA256SUMS.txt', '```', '',
              'FontCarver is `MIT OR Apache-2.0`. Bundled third-party notices ship inside '
              'every package.', '']
    return '\n'.join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tag', required=True, help='the release tag, such as v0.1.0-alpha.1')
    parser.add_argument('--check-tag', action='store_true',
                        help='only compare the tag with the working tree version')
    parser.add_argument('--source', type=Path, help='directory of downloaded platform artifacts')
    parser.add_argument('--destination', type=Path, help='directory to stage release assets in')
    parser.add_argument('--revision', help='the commit the tag points at')
    parser.add_argument('--run-url', default='', help='workflow run the notes should cite')
    parser.add_argument('--repo-url', default='',
                        help='repository the notes link documents in, pinned to the tag')
    parser.add_argument('--notes', type=Path, help='default: DESTINATION/../release-notes.md')
    parser.add_argument('--output', type=Path, help='write the staging report as JSON')
    args = parser.parse_args()
    try:
        version = tag_version(args.tag)
        declared = configured_version()
        if version != declared:
            raise ReleaseError(f'tag {args.tag} does not name the version this tree '
                               f'declares: {declared}')
        if args.check_tag:
            print(f'PASS: {args.tag} matches the configured version {declared}')
            return 0
        if not (args.source and args.destination and args.revision):
            raise ReleaseError('staging needs --source, --destination and --revision')
        report = stage(args.source.resolve(), args.destination.resolve(), version, args.revision)
        notes_path = args.notes or args.destination.resolve().parent / 'release-notes.md'
        notes_path.parent.mkdir(parents=True, exist_ok=True)
        notes_path.write_text(notes(report, args.run_url, args.repo_url, args.tag),
                              encoding='utf-8')
        report['notes'] = str(notes_path)
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    except (ReleaseError, ValueError, OSError, KeyError) as error:
        print(f'FAIL: {error}', file=sys.stderr)
        return 1
    for label, names in report['platforms'].items():
        print(f'{label}: {", ".join(names)}')
    print(f'PASS: staged {len(report["assets"])} assets for {args.tag} '
          f'({"pre-release" if report["prerelease"] else "release"})')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
