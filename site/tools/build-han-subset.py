#!/usr/bin/env python3
"""Subset the display Han face to the characters this site actually sets.

Shipping a Han family would cost more than every other byte on the site put
together, so the site ships exactly the characters it writes, at one weight,
and nothing else. Two subsets, because the two pages need different things:

  han-display.woff2  the handful of Han characters the English page sets, so
                     an English reader downloads a few kilobytes and not a
                     whole page's worth of a writing system they are not
                     reading;
  han-zh.woff2       every character the Chinese page sets, so that page is
                     typeset in a real Song serif rather than in whatever the
                     reader's system happens to fall back to.

Re-run this after changing Chinese copy; the site checker's weight budget is
what notices when you forget.

    python3 site/tools/build-han-subset.py

The subsets keep the copyright, licence and licence-URL entries the OFL
requires (name IDs 0, 13 and 14) and carry no family name at all, because
Source Han Serif reserves the name `Source` and these files are modified
versions of it. The site names them in CSS instead.

Needs fontTools with Brotli, and Source Han Serif. Both are development
dependencies: the built site does not need them.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_FONT = Path.home() / '.local/share/fonts/SourceHanSerif-VF.otf.ttf'
FONTS = ROOT / 'public/assets/fonts'
# Han, kana, CJK punctuation and the full-width forms: everything the
# @font-face's unicode-range claims, so the face is never asked for a glyph it
# does not carry.
HAN = re.compile(r'[⺀-鿿　-〿＀-￯]')
# What each subset has to cover. The Chinese page shows the same specimen
# switcher as the English one, so it needs both sets.
SCOPES = {
    'han-display': ['src/data/content.en.ts', 'src/data/specimens.ts', 'src/components'],
    'han-zh': ['src/data/content.en.ts', 'src/data/specimens.ts', 'src/components',
               'src/data/content.zh.ts'],
}


def characters(roots: list[str]) -> set[str]:
    """Every CJK character the named sources contain."""
    found: set[str] = set()
    for name in roots:
        path = ROOT / name
        files = sorted(path.rglob('*')) if path.is_dir() else [path]
        for item in files:
            if item.is_file() and item.suffix in {'.ts', '.astro', '.svelte'}:
                found.update(HAN.findall(item.read_text(encoding='utf-8')))
    return found


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--font', type=Path, default=DEFAULT_FONT,
                        help=f'source face (default: {DEFAULT_FONT})')
    parser.add_argument('--weight', default='500', help='the single weight to instance')
    parser.add_argument('--font-number', type=int, default=2,
                        help='face inside the collection; 2 is Source Han Serif SC VF')
    args = parser.parse_args()

    if not args.font.is_file():
        print(f'no source face at {args.font}; install Source Han Serif or pass --font',
              file=sys.stderr)
        return 1

    from fontTools.ttLib import TTFont

    with tempfile.TemporaryDirectory(prefix='han-subset-') as directory:
        # Source Han Serif ships as a collection of regional faces sharing one
        # glyph store; take the one face, then instance it, because a variable
        # Han face carries every weight of every glyph and the site sets one.
        single = Path(directory) / 'face.otf'
        face = TTFont(args.font, fontNumber=args.font_number)
        face.save(single)
        face.close()
        instance = Path(directory) / 'instance.otf'
        subprocess.run([sys.executable, '-m', 'fontTools.varLib.instancer',
                        str(single), f'wght={args.weight}',
                        '-o', str(instance)], check=True, stdout=subprocess.DEVNULL)

        FONTS.mkdir(parents=True, exist_ok=True)
        for name, roots in SCOPES.items():
            wanted = characters(roots)
            if not wanted:
                print(f'{name}: no CJK characters found in {roots}', file=sys.stderr)
                return 1
            output = FONTS / f'{name}.woff2'
            subprocess.run([sys.executable, '-m', 'fontTools.subset', str(instance),
                            '--text=' + ''.join(sorted(wanted)), '--layout-features=',
                            '--no-hinting', '--desubroutinize', '--name-IDs=0,13,14',
                            '--no-recalc-timestamp', '--flavor=woff2',
                            f'--output-file={output}'], check=True)
            print(f'{output.relative_to(ROOT)}: {len(wanted)} characters, '
                  f'{output.stat().st_size // 1024} KiB')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
