#!/usr/bin/env python3
"""Read-only disk/file-count budgets for local build and task artifacts.

Run before/after build batches and task completion. External task directories
must be listed explicitly with --task-temp; this script never deletes files and
does not scan unrelated home or temporary directories.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import stat
from pathlib import Path

GIB = 1024 ** 3
ROOT = Path(__file__).resolve().parents[1]


def inventory(path: Path, seen: set[tuple[int, int]]) -> tuple[int, int]:
    """Count allocated bytes and files, once per inode, without following links."""
    if not path.exists() and not path.is_symlink():
        return 0, 0
    if path.is_symlink():
        raise ValueError(f"artifact root is a symlink; check its destination explicitly: {path}")
    pending = [path]
    size = count = 0
    while pending:
        item = pending.pop()
        try:
            info = item.lstat()
        except FileNotFoundError:
            continue  # A running tool may finish and remove its temporary file.
        identity = (info.st_dev, info.st_ino)
        if identity in seen:
            continue
        seen.add(identity)
        size += getattr(info, 'st_blocks', (info.st_size + 511) // 512) * 512
        if stat.S_ISDIR(info.st_mode):
            with os.scandir(item) as entries:
                pending.extend(Path(entry.path) for entry in entries)
        else:
            count += 1
    return size, count


def check(args: argparse.Namespace) -> dict:
    groups = {
        'build': [ROOT / 'target', ROOT / 'src-tauri/target'] + args.build_dir,
        'temporary': [ROOT / name for name in (
            '.scratch', 'test-output', 'app/test-results', 'app/playwright-report',
        )] + args.task_temp,
    }
    seen: set[tuple[int, int]] = set()
    rows = []
    totals = {}
    errors = []
    warnings = []
    for group, paths in groups.items():
        size = count = 0
        for path in paths:
            try:
                allocated, files = inventory(path, seen)
            except (OSError, ValueError) as error:
                errors.append(str(error))
                continue
            size += allocated
            count += files
            rows.append(dict(group=group, path=str(path), bytes=allocated, files=files))
        totals[group] = dict(bytes=size, files=count)
    if totals['build']['bytes'] > args.build_limit_gib * GIB:
        errors.append('build artifacts exceed the capacity ceiling')
    elif totals['build']['bytes'] > 10 * GIB:
        warnings.append('build artifacts exceed 10 GiB; plan profile-cache cleanup')
    if totals['build']['files'] > 100_000:
        errors.append('build artifacts exceed 100,000 files')
    if totals['temporary']['bytes'] > args.temp_limit_gib * GIB:
        errors.append('temporary artifacts exceed the capacity ceiling')
    if totals['temporary']['files'] > 10_000:
        errors.append('temporary artifacts exceed 10,000 files')
    # External task directories can live on another filesystem. Check each mount.
    free = {}
    for path in [ROOT, *args.build_dir, *args.task_temp]:
        existing = path.absolute()
        while not existing.exists():
            existing = existing.parent
        available = shutil.disk_usage(existing).free
        free[str(path)] = available
        if available < args.min_free_gib * GIB:
            errors.append(f'less than {args.min_free_gib:g} GiB free at {path}')
    return dict(ok=not errors, totals=totals, paths=rows, free_bytes=free,
                warnings=warnings, errors=errors)


def nonnegative(value: str) -> float:
    number = float(value)
    if not 0 <= number < float('inf'):
        raise argparse.ArgumentTypeError('budget must be finite and nonnegative')
    return number


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--build-dir', type=Path, action='append', default=[],
                        help='additional configured Cargo target directory')
    parser.add_argument('--task-temp', type=Path, action='append', default=[])
    parser.add_argument('--build-limit-gib', type=nonnegative, default=20.)
    parser.add_argument('--temp-limit-gib', type=nonnegative, default=1.)
    parser.add_argument('--min-free-gib', type=nonnegative, default=20.)
    parser.add_argument('--json', action='store_true')
    args = parser.parse_args()
    if os.environ.get('CARGO_TARGET_DIR'):
        args.build_dir.append(Path(os.environ['CARGO_TARGET_DIR']).absolute())
    report = check(args)
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        for name, total in report['totals'].items():
            print(f"{name}: {total['bytes'] / GIB:.2f} GiB, {total['files']:,} files")
        print(f"workspace free: {report['free_bytes'][str(ROOT)] / GIB:.2f} GiB")
        for level in ['warnings', 'errors']:
            for message in report[level]:
                print(f'{level}: {message}')
        print('PASS' if report['ok'] else 'FAIL — pause builds and inspect disposable artifacts')
    return 0 if report['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
