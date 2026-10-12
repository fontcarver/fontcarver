#!/usr/bin/env python3
"""Install, verify and remove a FontCarver rpm inside a throwaway container.

The deb check (`check_deb_install.py`) is built around dpkg, apt and a bubblewrap
overlay; none of that transfers to rpm. A container supplies the same thing that
overlay did — a root filesystem this check may install into and then discard —
on a host that is not an rpm distribution at all.

    python3 scripts/check_rpm_install.py --artifact target/release/bundle/rpm/X.rpm \
        --version 0.1.0-alpha.1 --output .scratch/rpm-install.json

Verified: dnf resolves the package's declared dependencies, the payload lands
where the package says, `rpm -V` finds no altered file, the installed binary
starts far enough to refuse a missing display, and removal takes the payload
with it. Not verified: a graphical workflow, an upgrade between versions,
signing, or publication.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile

DEFAULT_IMAGE = 'fedora:41'
EXECUTABLE = '/usr/bin/fontcarver-desktop'


def command(arguments: list[str], *, timeout: int = 1800) -> str:
    """Run one command with a bounded lifetime and keep its diagnostics."""
    print('+ ' + ' '.join(arguments), flush=True)
    with tempfile.TemporaryFile(mode='w+') as output:
        process = subprocess.Popen(arguments, text=True, stdout=output,
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


def runtime() -> str:
    """Prefer rootless podman; fall back to docker. Name the gap, never crash."""
    for candidate in ('podman', 'docker'):
        if shutil.which(candidate):
            return candidate
    raise RuntimeError('rpm install verification needs podman or docker on PATH')


def container_command(engine: str, image: str, artifact: Path, script: str) -> list[str]:
    """Mount the candidate read-only; the container root is the disposable part."""
    return [engine, 'run', '--rm', '--network', 'bridge',
            '--volume', f'{artifact}:/candidate.rpm:ro',
            '--env', 'HOME=/root', '--env', 'TMPDIR=/tmp',
            '--env', 'FC_CONFIG_DIR=/root/.config/fontcarver',
            image, 'bash', '-eu']


def container_script(version: str) -> str:
    """The sequence run as root inside the container, in order, failing closed."""
    return f'''
dnf -y install /candidate.rpm
name=$(rpm -q --queryformat '%{{NAME}}' -p /candidate.rpm)
installed=$(rpm -q --queryformat '%{{NAME}} %{{VERSION}}' "$name")
echo "installed: $installed"
case "$installed" in
  *" {version}") ;;
  *) echo "unexpected installed version: $installed" >&2; exit 1 ;;
esac
test -x {EXECUTABLE}
rpm -ql "$name" | sed -n '/\\.desktop$/p' | while read -r desktop; do
  test -s "$desktop"
done
# An altered or missing payload file is a packaging defect, not a runtime one.
rpm -V "$name"
unset DISPLAY WAYLAND_DISPLAY
"{EXECUTABLE}" --version
set +e
"{EXECUTABLE}"
status=$?
set -e
test "$status" -ne 0
dnf -y remove "$name"
test ! -e {EXECUTABLE}
echo "rpm install check complete"
'''


CHECKS = ['dnf install with declared dependency resolution',
          'installed package name and version',
          'installed executable and desktop entries present',
          'rpm -V finds no altered payload file',
          '--version prints and the windowless launch refuses',
          'removal takes the executable with it']

LIMITS = ['no graphical workflow', 'no upgrade between versions',
          'no signature or notarization check', 'no publication',
          'the container, not a real desktop, supplies the dependencies']


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifact', type=Path, required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--image', default=DEFAULT_IMAGE)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    artifact = args.artifact.resolve(strict=True)
    engine = runtime()
    script = container_script(args.version)
    # The script is mounted rather than quoted into argv, so no shell on either
    # side of the container boundary has to re-parse it.
    with tempfile.NamedTemporaryFile('w', suffix='.sh', delete=False) as handle:
        handle.write(script)
        script_path = handle.name
    try:
        argv_run = container_command(engine, args.image, artifact, script)
        argv_run = argv_run[:-3] + ['--volume', f'{script_path}:/check.sh:ro',
                                    argv_run[-3], 'bash', '-eu', '/check.sh']
        output = command(argv_run, timeout=1800)
    finally:
        os.unlink(script_path)
    report = {'schema_version': 1, 'artifact': artifact.name, 'version': args.version,
              'image': args.image, 'engine': engine, 'checks': CHECKS, 'limits': LIMITS,
              'log_tail': output[-4000:]}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n',
                           encoding='utf-8')
    print(f'PASS: rpm install verified in {args.image} -> {args.output}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
