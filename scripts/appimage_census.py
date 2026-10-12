#!/usr/bin/env python3
"""Count the release verdicts of an AppImage payload's owning packages.

The census is a function of what the build host carries, not of what FontCarver
needs: the bundler sweeps what it finds. Re-run it per build host and per
release, from that build's own inventory, and treat an older census as stale.

    python3 scripts/appimage_census.py --inventory audit.json --output census.json

Exit status is 1 while any package still needs a determination, so a CI job can
collect the evidence and still report the work as outstanding.
"""

from __future__ import annotations

import argparse
import collections
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))

import appimage_licenses as licenses

BLOCKING = ('forbidden', 'ruling-required', 'unidentified')


def census(inventory: dict) -> dict:
    """Classify every owning package the inventory recorded."""
    packages = inventory.get('packages') or []
    if not packages:
        raise ValueError('inventory records no packages; nothing was classified')
    rows = []
    for entry in packages:
        name = entry.get('package') or ''
        labels = list(entry.get('labels') or [])
        verdict = licenses.verdict_for_package(name, labels)
        rows.append({'package': name, 'labels': labels, 'verdict': verdict['verdict'],
                     'permitted': verdict['permitted'], 'reason': verdict['reason']})
    counts = collections.Counter(row['verdict'] for row in rows)
    rows.sort(key=lambda row: (BLOCKING.index(row['verdict'])
                               if row['verdict'] in BLOCKING else len(BLOCKING),
                               row['package']))
    return {'schema_version': 1, 'artifact': inventory.get('artifact'),
            'counts': dict(counts), 'blocking': sum(counts[kind] for kind in BLOCKING),
            'packages': rows}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inventory', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    report = census(json.loads(args.inventory.read_text(encoding='utf-8')))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n',
                           encoding='utf-8')
    summary = ', '.join(f'{kind}={count}' for kind, count in sorted(report['counts'].items()))
    print(f'{len(report["packages"])} packages: {summary} -> {args.output}')
    for row in report['packages'][:20]:
        if row['verdict'] in BLOCKING:
            print(f'  {row["verdict"]:16s} {row["package"]} {row["labels"]}')
    return 1 if report['blocking'] else 0


if __name__ == '__main__':
    raise SystemExit(main())
