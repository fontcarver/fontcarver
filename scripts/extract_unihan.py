#!/usr/bin/env python3
"""Extract pinned Unicode 17 Unihan fields and the GB 18030-2022 font repertoire.

Inputs are downloaded separately; SHA-256 checks prevent accidental data upgrades.
See crates/fc-glyphdata/data/README.md for URLs and licenses.
"""

import argparse
import hashlib
from pathlib import Path
import re
import zipfile

UNIHAN_SHA = "f7a48b2b545acfaa77b2d607ae28747404ce02baefee16396c5d2d7a8ef34b5e"
GB_SHA = "50b16dfde5438f4c63ff59adcf3a5a6d8d3e6987f2b7c31e4383aaa5a6f6b6e8"
FIELDS = (
    "kRSUnicode",
    "kTotalStrokes",
    "kMandarin",
    "kCantonese",
    "kJapaneseOn",
    "kJapaneseKun",
    "kKorean",
    "kIICore",
)


def checked(path: Path, expected: str) -> bytes:
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != expected:
        raise ValueError(f"Unexpected source hash: {path}")
    return data


def unihan(path: Path, output: Path) -> None:
    checked(path, UNIHAN_SHA)
    records: dict[int, dict[str, str]] = {}
    with zipfile.ZipFile(path) as archive:
        for name in archive.namelist():
            for line in archive.read(name).decode("utf-8").splitlines():
                if not line.startswith("U+"):
                    continue
                scalar, key, value = line.split("\t", 2)
                if key in FIELDS:
                    cp = int(scalar[2:], 16)
                    row = records.setdefault(cp, {})
                    if key in row:
                        raise ValueError(f"Duplicate property: {scalar} {key}")
                    row[key] = value
    columns = "\t".join(FIELDS)
    header = (
        "# Unicode 17.0.0 Unihan subset; Copyright 2025 Unicode, Inc.\n"
        "# Unicode License v3; see UNICODE-LICENSE.txt.\n"
        "# Source: https://www.unicode.org/Public/17.0.0/ucd/Unihan.zip\n"
        f"# Source SHA-256: {UNIHAN_SHA}\n"
        f"# Columns: codepoint\t{columns}\n"
    )
    for plane in sorted({cp >> 16 for cp in records}):
        rows = [
            f"{cp:X}\t" + "\t".join(row.get(field, "") for field in FIELDS)
            for cp, row in sorted(records.items())
            if cp >> 16 == plane
        ]
        target = output / f"Unihan-17-plane{plane}.tsv"
        target.write_text(header + "\n".join(rows) + "\n", encoding="utf-8")
        if target.stat().st_size > 2_000_000:
            raise ValueError(f"Split source shard before committing: {target}")
        print(f"{target.name}: {len(rows)} records, {target.stat().st_size} bytes")


def gb_level1(path: Path, output: Path) -> None:
    data = checked(path, GB_SHA).decode("utf-8")
    # GB 18030-2022 section 9.2: assigned single/double-byte characters plus
    # Extension A through U+4DB5 and URO through U+9FEF (the 2022 repertoire).
    cps = set(range(0x3400, 0x4DB6)) | set(range(0x4E00, 0x9FF0))
    for line in data.splitlines():
        match = re.match(r"<U([0-9A-F]+)>\s+((?:\\x[0-9A-F]{2}){1,2})\s+\|0", line)
        if match:
            cp = int(match[1], 16)
            if not 0xE000 <= cp <= 0xF8FF:
                cps.add(cp)
    # Annex D.3 removes these nine formerly assigned compatibility characters.
    cps.difference_update(
        (0xF92C, 0xF979, 0xF995, 0xF9E7, 0xF9F1, 0xFA0C, 0xFA0D, 0xFA18, 0xFA20)
    )
    header = (
        "# GB 18030-2022 Level 1 assigned Unicode font repertoire.\n"
        "# Derived from ICU 77.1 gb18030-2022.ucm, Unicode License v3.\n"
        "# Copyright 2016 and later Unicode, Inc. and others;\n"
        "# Copyright 2000-2012 International Business Machines Corporation and others.\n"
        "# See UNICODE-LICENSE.txt and README.md for provenance and exclusions.\n"
        f"# Source SHA-256: {GB_SHA}\n"
    )
    (output / "GB18030-2022-Level1.txt").write_text(
        header + "\n".join(f"{cp:04X}" for cp in sorted(cps)) + "\n"
    )
    print(f"GB18030-2022-Level1.txt: {len(cps)} codepoints")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--unihan", type=Path, required=True)
    parser.add_argument("--gb18030", type=Path, required=True)
    parser.add_argument(
        "--output-dir", type=Path, default=Path("crates/fc-glyphdata/data")
    )
    args = parser.parse_args()
    unihan(args.unihan, args.output_dir)
    gb_level1(args.gb18030, args.output_dir)


if __name__ == "__main__":
    main()
