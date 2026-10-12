#!/usr/bin/env python3
"""Install/reinstall/remove a FontCarver deb in a disposable rootless overlay.

Linux only: requires user namespaces, overlayfs, bubblewrap, and already installed
runtime dependencies. APT has no network access and cannot change the host root.
This verifies package-manager behavior, not a graphical desktop workflow.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def command(arguments: list[str], *, env: dict | None = None, timeout: int = 180) -> str:
    """Run one command with a bounded lifetime and preserve its diagnostics."""
    print('+ ' + ' '.join(arguments), flush=True)
    # A descendant retaining stdout must not keep communicate() waiting after
    # its command exits. A private file also bounds timeout cleanup reliably.
    with tempfile.TemporaryFile(mode="w+") as output:
        process = subprocess.Popen(arguments, env=env, text=True, stdout=output,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        timed_out = False
        try:
            process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
        finally:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=5)
        output.seek(0)
        text = output.read()
    print(text, end='', flush=True)
    if timed_out:
        raise RuntimeError(f'{arguments[0]} timed out after {timeout} seconds')
    if process.returncode:
        raise RuntimeError(f'{arguments[0]} exited {process.returncode}')
    return text


def sandbox_command(scratch: Path, artifact: Path, version: str) -> list[str]:
    """Expose writable package paths only through task-owned overlay mounts."""
    args = ['bwrap', '--die-with-parent', '--unshare-net', '--unshare-ipc',
            '--unshare-uts', '--hostname', 'fontcarver-install-check',
            '--ro-bind', '/', '/']
    for name in ('usr', 'etc', 'var'):
        args += ['--bind', str(scratch / name / 'merged'), '/' + name]
    args += ['--tmpfs', '/tmp', '--tmpfs', '/run', '--tmpfs', str(ROOT / '.scratch'),
             '--proc', '/proc', '--dev', '/dev',
             '--ro-bind', str(artifact), '/tmp/fontcarver.deb',
             '--setenv', 'HOME', '/tmp/home', '--setenv', 'TMPDIR', '/tmp',
             '--setenv', 'FC_CONFIG_DIR', '/tmp/home/.config/fontcarver',
             '--unsetenv', 'DISPLAY', '--unsetenv', 'WAYLAND_DISPLAY',
             '--unsetenv', 'DBUS_SESSION_BUS_ADDRESS', '--unsetenv', 'XDG_RUNTIME_DIR',
             sys.executable, str(Path(__file__).resolve()), '--inside', 'install',
             '--version', version, '/tmp/fontcarver.deb']
    return args


def package_directories(data: bytes) -> list[PurePosixPath]:
    """Find narrow writable /usr parents without extracting or trusting archive paths."""
    directories = set()
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:") as archive:
        for member in archive:
            path = PurePosixPath(member.name)
            if path.is_absolute() or ".." in path.parts:
                raise RuntimeError(f"unsafe package path: {member.name}")
            if not path.parts:
                continue
            if path.parts[0] != "usr":
                raise RuntimeError(f"unexpected package destination outside /usr: {member.name}")
            parent = path if member.isdir() else path.parent
            if len(parent.parts) > 1:
                directories.add(PurePosixPath(*parent.parts[1:]))
    return sorted(directories)


def mounted_check(scratch: Path, artifact: Path, version: str) -> None:
    """Mount overlays inside the caller's private user/mount/PID namespace."""
    directories = package_directories(subprocess.check_output(
        ['dpkg-deb', '--fsys-tarfile', str(artifact)], timeout=60))
    for name in ('usr', 'etc', 'var'):
        base = scratch / name
        for part in ('upper', 'work', 'merged'):
            (base / part).mkdir(parents=True, exist_ok=True)
        # Linux checks write permissions before copy-up. The host's root-owned
        # lower directories are not owned by our namespace root, so prepare the
        # narrow writable package directories and mutable dpkg state ourselves.
        if name == 'usr':
            for directory in directories:
                (base / 'upper' / directory).mkdir(parents=True, exist_ok=True)
            for cache in ('share/applications/mimeinfo.cache', 'share/icons/hicolor/icon-theme.cache'):
                source = Path('/usr') / cache
                if source.is_file():
                    shutil.copyfile(source, base / 'upper' / cache)
        elif name == 'var':
            for directory in ('lib/dpkg/info', 'lib/dpkg/updates', 'lib/apt/lists/auxfiles', 'cache/apt', 'log/apt'):
                (base / 'upper' / directory).mkdir(parents=True, exist_ok=True)
            for filename in ('status', 'available'):
                source = Path('/var/lib/dpkg') / filename
                if source.is_file():
                    shutil.copyfile(source, base / 'upper/lib/dpkg' / filename)
            shutil.copytree('/var/lib/dpkg/triggers', base / 'upper/lib/dpkg/triggers',
                            copy_function=shutil.copyfile, ignore=shutil.ignore_patterns('Lock'))
            (base / 'upper/lib/dpkg/triggers/Lock').touch()
            for filename in ('lock', 'lock-frontend'):
                (base / 'upper/lib/dpkg' / filename).touch()
            for filename in ('dpkg.log', 'apt/history.log', 'apt/term.log'):
                (base / 'upper/log' / filename).touch()
        options = f'lowerdir=/{name},upperdir={base}/upper,workdir={base}/work'
        command(['mount', '-t', 'overlay', 'overlay', '-o', options, str(base / 'merged')])
        # overlayfs creates this mode-000 directory. Keep the shared artifact
        # checker able to traverse owned scratch while the namespace is alive.
        (base / 'work/work').chmod(0o700)
    command(sandbox_command(scratch, artifact, version), timeout=360)


def apt_arguments(action: str, package: str) -> list[str]:
    """Use private mounts, no downloads, no extra recommended dependencies."""
    return ['apt-get', '-y', '--no-download', '--no-install-recommends',
            '-o', 'APT::Sandbox::User=root', '-o', 'DPkg::Lock::Timeout=30',
            '-o', 'Dir::Etc::sourcelist=/tmp/apt-sources.list',
            '-o', 'Dir::Etc::sourceparts=/tmp/apt-sources.d',
            '-o', 'Dir::Cache::archives=/tmp/apt-archives',
            '-o', 'DPkg::Options::=--force-unsafe-io',
            *(['--reinstall'] if action == 'reinstall' else []),
            'install' if action == 'reinstall' else action, package]


def installed_check(artifact: Path, version: str) -> None:
    """Exercise real APT and dpkg state plus retained private user files."""
    executable = Path('/usr/bin/fontcarver-desktop')
    if executable.exists():
        raise RuntimeError('host already has FontCarver installed; use a clean runner')
    home = Path('/tmp/home')
    markers = [home / 'Documents/keep.fcf', home / '.config/fontcarver/preferences.json',
               home / '.config/fontcarver/autosave/keep.fcf']
    for path in markers:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('package-manager retention marker\n', encoding='utf-8')
    Path('/tmp/apt-archives/partial').mkdir(parents=True)
    architecture = command(['dpkg-deb', '-f', str(artifact), 'Architecture']).strip()
    shutil.copyfile(artifact, f'/tmp/apt-archives/font-carver_{version}_{architecture}.deb')
    Path('/tmp/apt-sources.list').touch()
    Path('/tmp/apt-sources.d').mkdir()
    Path('/tmp/apt.conf.d').mkdir()
    Path('/tmp/apt.conf').write_text(
        'Dir::Etc::parts "/tmp/apt.conf.d";\nDir::Etc::main "/tmp/apt.conf";\n',
        encoding='utf-8')
    env = dict(os.environ, DEBIAN_FRONTEND='noninteractive', APT_CONFIG='/tmp/apt.conf')
    for action in ('install', 'reinstall'):
        command(apt_arguments(action, str(artifact)), env=env)
        state = command(['dpkg-query', '-W', '-f=${Status}\n${Version}\n', 'font-carver'])
        if state.strip() != f'install ok installed\n{version}':
            raise RuntimeError(f'unexpected installed package state: {state!r}')
        for path in (executable, Path('/usr/share/doc/font-carver/copyright'),
                     Path('/usr/lib/FontCarver/INSTALL.md')):
            if not path.is_file() or not path.stat().st_size:
                raise RuntimeError(f'missing installed resource: {path}')
        command([sys.executable, str(ROOT / 'scripts/smoke_release_native.py'),
                 '--executable', str(executable), '--version', version, '--launch', 'headless',
                 '--report', '/tmp/smoke.json'])
        print(Path('/tmp/smoke.json').read_text(encoding='utf-8'), flush=True)
    command(apt_arguments('remove', 'font-carver'), env=env)
    if executable.exists() or Path('/usr/share/doc/font-carver/copyright').exists():
        raise RuntimeError('package files remain after APT removal')
    for path in markers:
        if path.read_text(encoding='utf-8') != 'package-manager retention marker\n':
            raise RuntimeError(f'user data changed during install/reinstall/removal: {path}')
    print('PASS: APT install, same-version reinstall, removal, and user-data retention', flush=True)


def clean_scratch(path: Path) -> None:
    """Remove only our private tree, including overlayfs mode-000 directories."""
    def readable(directory: Path) -> None:
        directory.chmod(0o700)
        for child in directory.iterdir():
            if child.is_dir() and not child.is_symlink():
                readable(child)
    readable(path)
    shutil.rmtree(path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('artifact', type=Path)
    parser.add_argument('--version', required=True)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--inside', choices=('mount', 'install'), help=argparse.SUPPRESS)
    parser.add_argument('--scratch', type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    artifact = args.artifact.resolve()
    if args.inside == 'mount':
        mounted_check(args.scratch, artifact, args.version)
        return 0
    if args.inside == 'install':
        installed_check(artifact, args.version)
        return 0
    report = {'schema_version': 1, 'ok': False, 'artifact': str(artifact),
              'version': args.version, 'host_platform': sys.platform,
              'isolation': 'private user/mount/PID namespaces; overlayfs; read-only host; no network',
              'checks': ['apt install', 'same-version reinstall', 'installed binary smoke',
                         'apt remove', 'private document/preferences/recovery retention'],
              'limits': ['Runtime dependencies come from the existing host; no clean-OS dependency resolution.',
                         'No graphical workflow, cross-version upgrade, signing, or publication.']}
    scratch = None
    try:
        if sys.platform != 'linux':
            raise RuntimeError('rootless APT installation verification requires Linux')
        for tool in ('unshare', 'mount', 'bwrap', 'apt-get', 'dpkg-deb'):
            if shutil.which(tool) is None:
                raise RuntimeError(f'required tool not found: {tool}')
        if not artifact.is_file():
            raise RuntimeError(f'installer not found: {artifact}')
        with artifact.open('rb') as stream:
            report['sha256'] = hashlib.file_digest(stream, 'sha256').hexdigest()
        package = command(['dpkg-deb', '-f', str(artifact), 'Package']).strip()
        version = command(['dpkg-deb', '-f', str(artifact), 'Version']).strip()
        if package != 'font-carver' or version != args.version:
            raise RuntimeError(f'unexpected package identity: {package} {version}')
        base = ROOT / '.scratch/p5f-w3-packaging'
        base.mkdir(parents=True, exist_ok=True)
        scratch = Path(tempfile.mkdtemp(prefix='install-', dir=base))
        command(['unshare', '--user', '--map-root-user', '--mount', '--pid', '--mount-proc', '--fork',
                 '--kill-child', sys.executable, str(Path(__file__).resolve()),
                 '--inside', 'mount', '--scratch', str(scratch), '--version', args.version,
                 str(artifact)], timeout=420)
        report['ok'] = True
    except (RuntimeError, OSError, subprocess.TimeoutExpired) as error:
        report['error'] = str(error)
    finally:
        if scratch is not None:
            clean_scratch(scratch)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(report, indent=2))
    return 0 if report['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
