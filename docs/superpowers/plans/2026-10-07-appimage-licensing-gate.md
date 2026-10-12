# AppImage Licensing Gate Implementation Plan (Plan A)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the blanket copyleft ban with a gate that permits LGPL in a bundled AppImage payload while hard-failing GPL and unidentifiable licenses, and generate the LGPL-2.1 §6 notices the image must carry — so the stock Tauri AppImage can ship.

**Architecture:** A new pure-logic module `scripts/appimage_licenses.py` classifies Debian license labels and renders the notice files. `scripts/appimage_inventory.py` keeps its single responsibility (walk the payload, resolve host package ownership) and gains a notice-writing mode that calls the new module. `scripts/appimage_policy.py` is rewritten from a thin-recipe receipt verifier into a payload scan that calls the classifier and names every offending file. The thin-AppImage builder is deleted.

**Tech Stack:** Python 3.12 stdlib only (`pathlib`, `re`, `subprocess`, `unittest`). No new dependencies. Tests are `unittest` modules under `scripts/`, run as `python3 scripts/test_<name>.py` from the `scripts/` directory, matching the existing convention.

**Spec:** `docs/superpowers/specs/2026-10-07-release-channels-design.md` (Plan A = spec §3, §4, §5, and the licensing parts of §7)

## Global Constraints

- Python stdlib only. No new Rust or npm dependencies in this plan.
- Error style: raise `ValueError` with a message naming the offending path or
  package. Existing scripts rely on `except (OSError, ValueError, subprocess.SubprocessError)`.
- Never execute the inspected AppImage or any payload binary. Read bytes only.
- Host dpkg data is a **hint, not build provenance**. Every report keeps that
  caveat verbatim; it is already in `inventory()['provenance_limit']`.
- Permitted in a native payload: LGPL (any version) and permissive licenses.
  **Forbidden: GPL, AGPL, SSPL.** An unidentified license is forbidden until identified.
- A GPL label does not automatically mean a GPL binary — Debian labels describe
  source packages. Such a package requires a recorded ruling, never a silent pass.
- Do not add a `bundle.linux.appimage` block to `tauri.conf.json`; its absence is
  what keeps `bundleMediaFramework` (GStreamer, the one GPL vector Tauri warns
  about) disabled.
- Run `python3 scripts/check_artifacts.py` before reporting any task complete.

## Review Focus

1. **`LGPL-2.1+` must not be classified as GPL.** The string `GPL` is a substring of `LGPL`; a naive match blocks every GTK and WebKit package and makes the gate useless. Pinned in Task 1.
2. **`GPL-3+ with autoconf exception` must still require a ruling.** A license exception in the label text must not downgrade the verdict to a pass. Pinned in Task 1.
3. **A package whose labels are all unrecognized but present (`public-domain`, `Expat`) must pass, not be treated as unidentified.** Unidentified means *no labels at all*; conflating the two blocks shippable images. Pinned in Task 1.
4. **A bundled `.so` with no owning package at all must fail and be named.** `inventory()` currently only counts these in `summary.shared_objects_without_package_candidate`; nothing fails. Pinned in Task 4.
5. **A notice that does not account for every `.so` found must fail.** Otherwise a payload change after notice generation ships an image whose notice is stale and incomplete. Pinned in Task 4.

---

### Task 1: License classifier

**Files:**
- Create: `scripts/appimage_licenses.py`
- Test: `scripts/test_appimage_licenses.py`

**Interfaces:**
- Consumes: nothing.
- Produces: `classify_labels(labels: list[str]) -> dict` returning
  `{'verdict': 'pass'|'ruling-required'|'forbidden'|'unidentified', 'kinds': list[str], 'requires_lgpl_notice': bool}`.
  `KIND_LGPL`, `KIND_GPL`, `KIND_FORBIDDEN`, `KIND_OTHER` string constants.
  `RULINGS: dict[str, str]` mapping package name to the reason a GPL-labelled
  package is permitted.

- [ ] **Step 1: Write the failing test**

```python
"""License classification for libraries a native bundler copied into an image."""
import unittest

import appimage_licenses as licenses


class ClassifyTests(unittest.TestCase):
    def test_lgpl_is_permitted_and_is_not_read_as_gpl(self):
        for label in ('LGPL-2.1', 'LGPL-2.1+', 'LGPL-2', 'lgpl-3'):
            with self.subTest(label=label):
                result = licenses.classify_labels([label])
                self.assertEqual(result['verdict'], 'pass')
                self.assertEqual(result['kinds'], [licenses.KIND_LGPL])
                self.assertTrue(result['requires_lgpl_notice'])

    def test_gpl_requires_a_recorded_ruling(self):
        for label in ('GPL-2', 'GPL-2+', 'GPL-3+ with autoconf exception'):
            with self.subTest(label=label):
                result = licenses.classify_labels([label])
                self.assertEqual(result['verdict'], 'ruling-required')
                self.assertIn(licenses.KIND_GPL, result['kinds'])

    def test_agpl_and_sspl_are_forbidden_outright(self):
        for label in ('AGPL-3', 'AGPL-3+', 'SSPL-1.0'):
            with self.subTest(label=label):
                self.assertEqual(licenses.classify_labels([label])['verdict'], 'forbidden')

    def test_mixed_gpl_and_lgpl_requires_a_ruling(self):
        # libfuse: `Files: *` is GPL-2 while the shipped `lib/*` is LGPL-2.1.
        result = licenses.classify_labels(['GPL-2', 'LGPL-2.1'])
        self.assertEqual(result['verdict'], 'ruling-required')
        self.assertTrue(result['requires_lgpl_notice'])

    def test_unrecognized_but_present_labels_pass_without_notice(self):
        result = licenses.classify_labels(['public-domain', 'Expat', 'BSD-3-clause'])
        self.assertEqual(result['verdict'], 'pass')
        self.assertEqual(result['kinds'], [licenses.KIND_OTHER])
        self.assertFalse(result['requires_lgpl_notice'])

    def test_no_labels_is_unidentified_not_pass(self):
        self.assertEqual(licenses.classify_labels([])['verdict'], 'unidentified')


if __name__ == '__main__':
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd scripts && python3 test_appimage_licenses.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'appimage_licenses'`

- [ ] **Step 3: Write minimal implementation**

```python
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd scripts && python3 test_appimage_licenses.py -v`
Expected: PASS, 6 tests

- [ ] **Step 5: Commit**

```bash
git add scripts/appimage_licenses.py scripts/test_appimage_licenses.py
git commit -m "feat(release): classify bundled-library licenses, separating LGPL from GPL"
```

---

### Task 2: Package verdicts and the notice renderer

**Files:**
- Modify: `scripts/appimage_licenses.py`
- Test: `scripts/test_appimage_licenses.py`

**Interfaces:**
- Consumes: `classify_labels`, `RULINGS`, `KIND_LGPL` from Task 1.
- Produces:
  `verdict_for_package(name: str, labels: list[str]) -> dict` adding `'permitted': bool` and `'reason': str | None` by consulting `RULINGS`.
  `render_bundled_libraries(entries: list[dict]) -> str` where each entry is
  `{'path': str, 'package': str, 'version': str, 'source_package': str, 'source_version': str, 'labels': list[str]}`.
  `source_reference(source_package: str, source_version: str) -> str`.

- [ ] **Step 1: Write the failing test**

```python
class PackageVerdictTests(unittest.TestCase):
    def test_gpl_package_passes_only_with_a_ruling(self):
        blocked = licenses.verdict_for_package('libfuse3-4', ['GPL-2', 'LGPL-2.1'])
        self.assertFalse(blocked['permitted'])
        self.assertIsNone(blocked['reason'])
        with unittest.mock.patch.dict(
                licenses.RULINGS,
                {'libfuse3-4': 'shipped lib/* is LGPL-2.1; GPL-2 covers unshipped utilities'}):
            allowed = licenses.verdict_for_package('libfuse3-4', ['GPL-2', 'LGPL-2.1'])
        self.assertTrue(allowed['permitted'])
        self.assertIn('LGPL-2.1', allowed['reason'])

    def test_forbidden_label_cannot_be_ruled_around(self):
        with unittest.mock.patch.dict(licenses.RULINGS, {'thing': 'we really want it'}):
            result = licenses.verdict_for_package('thing', ['AGPL-3'])
        self.assertFalse(result['permitted'])

    def test_lgpl_and_unidentified_need_no_ruling_and_differ(self):
        self.assertTrue(licenses.verdict_for_package('libgtk-3-0', ['LGPL-2.1'])['permitted'])
        self.assertFalse(licenses.verdict_for_package('mystery', [])['permitted'])


class NoticeTests(unittest.TestCase):
    def test_notice_lists_every_library_with_its_source_reference(self):
        text = licenses.render_bundled_libraries([
            {'path': 'usr/lib/libgtk-3.so.0', 'package': 'libgtk-3-0t64',
             'version': '3.24.49-1', 'source_package': 'gtk+3.0',
             'source_version': '3.24.49-1', 'labels': ['LGPL-2.1']},
        ])
        self.assertIn('usr/lib/libgtk-3.so.0', text)
        self.assertIn('libgtk-3-0t64 3.24.49-1', text)
        self.assertIn('LGPL-2.1', text)
        self.assertIn(licenses.source_reference('gtk+3.0', '3.24.49-1'), text)
        self.assertIn('LGPL-2.1 text', text)

    def test_notice_refuses_to_render_an_empty_inventory(self):
        with self.assertRaisesRegex(ValueError, 'no bundled libraries'):
            licenses.render_bundled_libraries([])
```

Add `import unittest.mock` to the test module's imports.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd scripts && python3 test_appimage_licenses.py -v`
Expected: FAIL with `AttributeError: module 'appimage_licenses' has no attribute 'verdict_for_package'`

- [ ] **Step 3: Write minimal implementation**

Append to `scripts/appimage_licenses.py`:

```python
SOURCE_BASE = 'https://sources.debian.org/src'


def source_reference(source_package: str, source_version: str) -> str:
    """Point at the source for one exact package version, for LGPL-2.1 section 6."""
    if not source_package or not source_version:
        raise ValueError('source reference needs a package and a version')
    return f'{SOURCE_BASE}/{source_package}/{source_version}/'


def verdict_for_package(name: str, labels: list[str]) -> dict:
    """Apply recorded rulings on top of a label classification."""
    result = dict(classify_labels(labels), package=name, permitted=False, reason=None)
    if result['verdict'] == 'pass':
        result['permitted'] = True
    elif result['verdict'] == 'ruling-required':
        reason = RULINGS.get(name)
        if reason:
            result.update(permitted=True, reason=reason)
    return result


def render_bundled_libraries(entries: list[dict]) -> str:
    """Render the per-library notice that discharges the naming obligation."""
    if not entries:
        raise ValueError('cannot render a notice with no bundled libraries')
    lines = [
        'Bundled shared libraries in this AppImage',
        '',
        'This image carries copies of system libraries. They remain under their own',
        'licenses; including them does not change FontCarver\'s MIT OR Apache-2.0 terms.',
        'Libraries under the LGPL may be replaced: extract the image, substitute an',
        'interface-compatible library, and repack. The LGPL-2 and LGPL-2.1 text',
        'accompanies this notice.',
        '',
        'Package names and versions describe the build host and are not a verified',
        'build provenance record. Source links address LGPL-2.1 section 6.',
        '',
    ]
    for entry in sorted(entries, key=lambda item: item['path']):
        lines.append(f'{entry["path"]}')
        lines.append(f'    package: {entry["package"]} {entry["version"]}')
        lines.append(f'    license: {", ".join(entry["labels"]) or "UNIDENTIFIED"}')
        lines.append(f'    source:  {source_reference(entry["source_package"], entry["source_version"])}')
    lines.append('')
    lines.append('Full license texts accompany this notice: LGPL-2 text, LGPL-2.1 text.')
    return '\n'.join(lines) + '\n'
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd scripts && python3 test_appimage_licenses.py -v`
Expected: PASS, 11 tests

- [ ] **Step 5: Commit**

```bash
git add scripts/appimage_licenses.py scripts/test_appimage_licenses.py
git commit -m "feat(release): render bundled-library notices with source references"
```

---

### Task 3: Carry source package and version into inventory notices

**Files:**
- Modify: `scripts/appimage_inventory.py:137-158` (`package_notices`) and `:160-190` (`inventory`)
- Test: `scripts/test_appimage_inventory.py`

**Interfaces:**
- Consumes: `host_inventory` → `(owners, packages)` where `packages[name]` is
  `{'version': str, 'source_package': str, 'source_version': str}`.
- Produces: `package_notices(root, rows, packages)` — note the **new third
  parameter** — with each entry gaining `'version'`, `'source_package'` and
  `'source_version'` keys. Task 4 relies on these.

- [ ] **Step 1: Write the failing test**

```python
    def test_package_notices_carry_source_package_and_version(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rows = [{'path': 'usr/lib/libgtk-3.so.0', 'type': 'file', 'elf_machine': 62,
                     'host_candidates': [{'package': 'libgtk-3-0t64', 'path': '/usr/lib/libgtk-3.so.0'}]}]
            packages = {'libgtk-3-0t64': {'version': '3.24.49-1',
                                          'source_package': 'gtk+3.0',
                                          'source_version': '3.24.49-1'}}
            entries = inventory.package_notices(root, rows, packages)
            self.assertEqual(len(entries), 1)
            self.assertEqual(entries[0]['package'], 'libgtk-3-0t64')
            self.assertEqual(entries[0]['version'], '3.24.49-1')
            self.assertEqual(entries[0]['source_package'], 'gtk+3.0')
            self.assertEqual(entries[0]['source_version'], '3.24.49-1')

    def test_package_notices_tolerate_an_unknown_package(self):
        with tempfile.TemporaryDirectory() as directory:
            rows = [{'path': 'usr/lib/libmystery.so.1', 'type': 'file', 'elf_machine': 62,
                     'host_candidates': [{'package': 'ghost', 'path': '/usr/lib/libmystery.so.1'}]}]
            entries = inventory.package_notices(Path(directory), rows, {})
            self.assertEqual(entries[0]['version'], '')
            self.assertEqual(entries[0]['source_package'], '')
```

Match the existing module alias in `scripts/test_appimage_inventory.py`; if it
imports the module under a different name, use that name instead of `inventory`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd scripts && python3 test_appimage_inventory.py -v`
Expected: FAIL with `TypeError: package_notices() takes 2 positional arguments but 3 were given`

- [ ] **Step 3: Write minimal implementation**

Change the signature and add the three keys:

```python
def package_notices(root: Path, rows: list[dict], packages: dict) -> list[dict]:
    """Record current-host copyright hashes and notices actually in the image."""
    result = []
    names = sorted({candidate['package'] for row in rows
                    for candidate in row.get('host_candidates', [])})
    for name in names:
        base = name.split(':', 1)[0]
        copyright_path = Path('/usr/share/doc') / base / 'copyright'
        source = packages.get(name, {})
        entry = {'package': name, 'host_copyright_path': str(copyright_path),
                 'version': source.get('version', ''),
                 'source_package': source.get('source_package', ''),
                 'source_version': source.get('source_version', '')}
```

Leave the rest of the function body unchanged. Then update the single call site in
`inventory()`:

```python
    notices = package_notices(payload, rows, packages)
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd scripts && python3 test_appimage_inventory.py -v`
Expected: PASS, existing tests plus the 2 new ones

- [ ] **Step 5: Commit**

```bash
git add scripts/appimage_inventory.py scripts/test_appimage_inventory.py
git commit -m "feat(release): record source package and version per bundled library"
```

---

### Task 4: Rewrite the policy gate

**Files:**
- Rewrite: `scripts/appimage_policy.py` (176 lines today; the thin-recipe receipt verification is removed wholesale)
- Create: `scripts/test_appimage_policy.py`

**Interfaces:**
- Consumes: `appimage_licenses.verdict_for_package`, `appimage_licenses.render_bundled_libraries`,
  `appimage_inventory.payload_files`, `appimage_inventory.host_inventory`,
  `appimage_inventory.annotate_host_candidates`, `appimage_inventory.package_notices` (3-arg form from Task 3).
- Produces: `require_releasable_appimage(payload: Path | None = None, artifact: Path | None = None) -> dict`
  — same name and signature `check_release_bundle.py` already imports, so that
  call site does not change. Raises `ValueError` naming offenders.
  `NOTICE_DIR = 'usr/lib/FontCarver/licenses/appimage'`.

- [ ] **Step 1: Write the failing test**

```python
"""Release gate tests for a bundled-library AppImage payload."""
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import appimage_policy as policy


def _payload(root: Path, libraries: dict[str, str]) -> None:
    """Create a payload with one application ELF plus the named shared objects."""
    binary = root / 'usr/bin/fontcarver-desktop'
    binary.parent.mkdir(parents=True, exist_ok=True)
    binary.write_bytes(b'\x7fELF\x02\x01\x01' + b'\x00' * 13)
    for name in libraries:
        target = root / 'usr/lib' / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(b'\x7fELF\x02\x01\x01' + b'\x00' * 13)


def _notices(root: Path, listed: str, texts: bool = True) -> None:
    """Create the notice directory the gate requires, with optional LGPL texts."""
    target = root / policy.NOTICE_DIR
    target.mkdir(parents=True, exist_ok=True)
    (target / policy.NOTICE_NAME).write_text(listed)
    if texts:
        for name in policy.REQUIRED_TEXTS:
            (target / name).write_text('license text')


class GateTests(unittest.TestCase):
    def test_missing_arguments_are_refused_without_touching_the_disk(self):
        with self.assertRaisesRegex(ValueError, 'payload and artifact'):
            policy.require_releasable_appimage()

    def test_lgpl_payload_passes_and_reports_every_library(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'libgtk-3.so.0': 'libgtk-3-0t64'})
            _notices(root, 'usr/lib/libgtk-3.so.0\n')
            with mock.patch.object(policy, 'resolve_libraries', return_value=[
                    {'path': 'usr/lib/libgtk-3.so.0', 'package': 'libgtk-3-0t64',
                     'version': '3.24.49-1', 'source_package': 'gtk+3.0',
                     'source_version': '3.24.49-1', 'labels': ['LGPL-2.1']}]):
                report = policy.require_releasable_appimage(root, root)
            self.assertEqual(report['bundled_shared_libraries'], 1)
            self.assertEqual(report['forbidden'], [])

    def test_missing_lgpl_text_fails_even_when_the_notice_is_complete(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'libgtk-3.so.0': 'libgtk-3-0t64'})
            _notices(root, 'usr/lib/libgtk-3.so.0\n', texts=False)
            with mock.patch.object(policy, 'resolve_libraries', return_value=[
                    {'path': 'usr/lib/libgtk-3.so.0', 'package': 'libgtk-3-0t64',
                     'version': '3.24.49-1', 'source_package': 'gtk+3.0',
                     'source_version': '3.24.49-1', 'labels': ['LGPL-2.1']}]):
                with self.assertRaisesRegex(ValueError, 'HOST-LGPL-2.txt'):
                    policy.require_releasable_appimage(root, root)

    def test_gpl_library_without_a_ruling_fails_and_is_named(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'libthing.so.1': 'thing'})
            _notices(root, 'usr/lib/libthing.so.1\n')
            with mock.patch.object(policy, 'resolve_libraries', return_value=[
                    {'path': 'usr/lib/libthing.so.1', 'package': 'thing',
                     'version': '1', 'source_package': 'thing', 'source_version': '1',
                     'labels': ['GPL-3']}]):
                with self.assertRaisesRegex(ValueError, 'libthing.so.1'):
                    policy.require_releasable_appimage(root, root)

    def test_library_with_no_owning_package_fails_and_is_named(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'liborphan.so.2': ''})
            _notices(root, 'usr/lib/liborphan.so.2\n')
            with mock.patch.object(policy, 'resolve_libraries', return_value=[
                    {'path': 'usr/lib/liborphan.so.2', 'package': '', 'version': '',
                     'source_package': '', 'source_version': '', 'labels': []}]):
                with self.assertRaisesRegex(ValueError, 'liborphan.so.2'):
                    policy.require_releasable_appimage(root, root)

    def test_notice_missing_a_library_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'libgtk-3.so.0': 'libgtk-3-0t64'})
            _notices(root, 'nothing here\n')
            with mock.patch.object(policy, 'resolve_libraries', return_value=[
                    {'path': 'usr/lib/libgtk-3.so.0', 'package': 'libgtk-3-0t64',
                     'version': '3.24.49-1', 'source_package': 'gtk+3.0',
                     'source_version': '3.24.49-1', 'labels': ['LGPL-2.1']}]):
                with self.assertRaisesRegex(ValueError, 'notice does not account'):
                    policy.require_releasable_appimage(root, root)

    def test_absent_notice_directory_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            root.mkdir()
            _payload(root, {'libgtk-3.so.0': 'libgtk-3-0t64'})
            with mock.patch.object(policy, 'resolve_libraries', return_value=[
                    {'path': 'usr/lib/libgtk-3.so.0', 'package': 'libgtk-3-0t64',
                     'version': '3.24.49-1', 'source_package': 'gtk+3.0',
                     'source_version': '3.24.49-1', 'labels': ['LGPL-2.1']}]):
                with self.assertRaisesRegex(ValueError, 'BUNDLED-LIBRARIES.txt'):
                    policy.require_releasable_appimage(root, root)


if __name__ == '__main__':
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd scripts && python3 test_appimage_policy.py -v`
Expected: FAIL — `resolve_libraries` does not exist and the gate still demands a thin receipt

- [ ] **Step 3: Write minimal implementation**

Replace the whole of `scripts/appimage_policy.py`:

```python
#!/usr/bin/env python3
"""Release decision for an AppImage whose bundler copied in system libraries.

LGPL libraries may ship with the notices that LGPL-2.1 section 6 requires. GPL,
AGPL and SSPL may not, and a library whose license cannot be identified blocks
the release until it is. Debian labels describe source packages, so a GPL label
demands a recorded ruling rather than an automatic refusal or an automatic pass.
This module never executes the artifact or any payload binary.
"""
from __future__ import annotations

from pathlib import Path

import appimage_inventory as inv
from appimage_licenses import verdict_for_package

NOTICE_DIR = 'usr/lib/FontCarver/licenses/appimage'
NOTICE_NAME = 'BUNDLED-LIBRARIES.txt'
REQUIRED_TEXTS = ('HOST-LGPL-2.txt', 'HOST-LGPL-2.1.txt')


def resolve_libraries(payload: Path) -> list[dict]:
    """List every bundled shared object with its host package and license labels."""
    rows = inv.payload_files(payload)
    libraries = [row for row in rows
                 if 'elf_machine' in row and '.so' in Path(row['path']).name]
    if not libraries:
        return []
    owners, packages = inv.host_inventory({Path(row['path']).name for row in libraries})
    inv.annotate_host_candidates(rows, owners, packages)
    notices = {entry['package']: entry
               for entry in inv.package_notices(payload, rows, packages)}
    resolved = []
    for row in libraries:
        candidates = row.get('host_candidates', [])
        entry = notices.get(candidates[0]['package']) if candidates else None
        resolved.append({
            'path': row['path'],
            'package': entry['package'] if entry else '',
            'version': entry.get('version', '') if entry else '',
            'source_package': entry.get('source_package', '') if entry else '',
            'source_version': entry.get('source_version', '') if entry else '',
            'labels': entry.get('labels', []) if entry else [],
        })
    return resolved


def require_releasable_appimage(payload: Path | None = None,
                                artifact: Path | None = None) -> dict:
    """Accept only a payload whose every bundled library is permitted and noticed.

    There is intentionally no environment-variable override or self-attested
    approval flag. Host package data is a hint about this inspection host, not a
    verified build provenance record.
    """
    if payload is None or artifact is None:
        raise ValueError('AppImage release needs both a payload and artifact path')
    root = payload.resolve(strict=True)
    libraries = resolve_libraries(root)
    forbidden = []
    for item in libraries:
        verdict = verdict_for_package(item['package'], item['labels'])
        if not verdict['permitted']:
            forbidden.append(f'{item["path"]} ({item["package"] or "no owning package"}: '
                             f'{", ".join(item["labels"]) or "unidentified"})')
    if forbidden:
        raise ValueError('AppImage release is blocked by bundled libraries: '
                         + '; '.join(sorted(forbidden)))
    notice = root / NOTICE_DIR / NOTICE_NAME
    if not notice.is_file():
        raise ValueError(f'AppImage release is blocked: missing {NOTICE_DIR}/{NOTICE_NAME}')
    text = notice.read_text(encoding='utf-8', errors='replace')
    missing = sorted(item['path'] for item in libraries if item['path'] not in text)
    if missing:
        raise ValueError('AppImage notice does not account for: ' + '; '.join(missing))
    for name in REQUIRED_TEXTS:
        if not (root / NOTICE_DIR / name).is_file():
            raise ValueError(f'AppImage release is blocked: missing {NOTICE_DIR}/{name}')
    return {'bundled_shared_libraries': len(libraries), 'forbidden': [],
            'notice': f'{NOTICE_DIR}/{NOTICE_NAME}',
            'scope': 'Bundled-library licenses from host package labels; '
                     'host data is a hint, not verified build provenance.'}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd scripts && python3 test_appimage_policy.py -v`
Expected: PASS, 7 tests

- [ ] **Step 5: Commit**

```bash
git add scripts/appimage_policy.py scripts/test_appimage_policy.py
git commit -m "feat(release): gate AppImage on bundled-library licenses, permitting LGPL"
```

---

### Task 5: Write the notice files during the build

**Files:**
- Modify: `scripts/appimage_inventory.py` (`main`, add `--notice-dir`)
- Test: `scripts/test_appimage_inventory.py`

**Interfaces:**
- Consumes: `appimage_licenses.render_bundled_libraries`, `appimage_policy.resolve_libraries`,
  `appimage_policy.NOTICE_DIR`, `NOTICE_NAME`, `REQUIRED_TEXTS`.
- Produces: `write_notices(payload: Path, repo_root: Path) -> list[str]` returning
  the relative paths written. Called by Task 6's build step.

- [ ] **Step 1: Write the failing test**

```python
    def test_write_notices_emits_notice_and_both_lgpl_texts(self):
        from unittest import mock
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'squashfs-root'
            (root / 'usr/lib').mkdir(parents=True)
            repo = Path(directory) / 'repo'
            texts = repo / 'scripts/release_licenses/native'
            texts.mkdir(parents=True)
            (texts / 'HOST-LGPL-2.txt').write_text('lgpl2')
            (texts / 'HOST-LGPL-2.1.txt').write_text('lgpl21')
            with mock.patch('appimage_policy.resolve_libraries', return_value=[
                    {'path': 'usr/lib/libgtk-3.so.0', 'package': 'libgtk-3-0t64',
                     'version': '3.24.49-1', 'source_package': 'gtk+3.0',
                     'source_version': '3.24.49-1', 'labels': ['LGPL-2.1']}]):
                written = inventory.write_notices(root, repo)
            notice = root / 'usr/lib/FontCarver/licenses/appimage/BUNDLED-LIBRARIES.txt'
            self.assertTrue(notice.is_file())
            self.assertIn('usr/lib/libgtk-3.so.0', notice.read_text())
            self.assertEqual((root / 'usr/lib/FontCarver/licenses/appimage/HOST-LGPL-2.txt'
                              ).read_text(), 'lgpl2')
            self.assertEqual(len(written), 3)
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd scripts && python3 test_appimage_inventory.py -v`
Expected: FAIL with `AttributeError: module 'appimage_inventory' has no attribute 'write_notices'`

- [ ] **Step 3: Write minimal implementation**

Add to `scripts/appimage_inventory.py` (import inside the function to avoid a
circular import, since `appimage_policy` imports this module):

```python
def write_notices(payload: Path, repo_root: Path) -> list[str]:
    """Write the bundled-library notice and LGPL texts into the payload."""
    import appimage_policy as policy
    from appimage_licenses import render_bundled_libraries
    libraries = policy.resolve_libraries(payload)
    target = payload / policy.NOTICE_DIR
    target.mkdir(parents=True, exist_ok=True)
    written = []
    notice = target / policy.NOTICE_NAME
    notice.write_text(render_bundled_libraries(libraries), encoding='utf-8')
    written.append(notice.relative_to(payload).as_posix())
    for name in policy.REQUIRED_TEXTS:
        source = repo_root / 'scripts/release_licenses/native' / name
        destination = target / name
        destination.write_text(source.read_text(encoding='utf-8'), encoding='utf-8')
        written.append(destination.relative_to(payload).as_posix())
    return written
```

Add to `main()`'s parser and body:

```python
    parser.add_argument('--notice-dir', action='store_true',
                        help='write LGPL notices into the payload instead of only reporting')
```

```python
        if args.notice_dir:
            for name in write_notices(args.payload.resolve(strict=True), Path(__file__).resolve().parents[1]):
                print(f'wrote {name}')
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd scripts && python3 test_appimage_inventory.py -v && python3 test_appimage_policy.py -v && python3 test_appimage_licenses.py -v`
Expected: PASS, all three modules

- [ ] **Step 5: Commit**

```bash
git add scripts/appimage_inventory.py scripts/test_appimage_inventory.py
git commit -m "feat(release): write bundled-library notices into the AppImage payload"
```

---

### Task 6: Delete the thin recipe and enable the stock AppImage

**Files:**
- Delete: `scripts/build_thin_appimage.py`, `scripts/test_build_thin_appimage.py`
- Modify: `scripts/build_release.py:30` (`native_bundles`), and the thin-builder branch plus `--appimage-sysroot` argument
- Modify: `scripts/check_release_bundle.py:24` (import) if it still references removed names
- Test: `scripts/test_build_release.py`

**Interfaces:**
- Consumes: `appimage_policy.require_releasable_appimage` (Task 4 signature, unchanged).
- Produces: `native_bundles('Linux') == ['deb', 'appimage']`. Plan B adds `'rpm'`.

- [ ] **Step 1: Write the failing test**

```python
    def test_linux_builds_deb_and_appimage_without_a_thin_recipe(self):
        self.assertEqual(build_release.native_bundles('Linux'), ['deb', 'appimage'])
        self.assertEqual(build_release.native_bundles('Windows'), ['msi'])

    def test_thin_appimage_builder_is_gone(self):
        from pathlib import Path
        root = Path(build_release.__file__).resolve().parent
        self.assertFalse((root / 'build_thin_appimage.py').exists())
```

Match the existing module alias in `scripts/test_build_release.py`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd scripts && python3 test_build_release.py -v`
Expected: FAIL — `native_bundles('Linux')` returns `['deb']`, and the builder file still exists

- [ ] **Step 3: Write minimal implementation**

```bash
git rm scripts/build_thin_appimage.py scripts/test_build_thin_appimage.py
```

In `scripts/build_release.py:30`:

```python
    bundles = {"Linux": ["deb", "appimage"], "Darwin": ["dmg"], "Windows": ["msi"]}
```

Then delete, in the same file: the `--appimage-sysroot` argument, the
`allowed = native + (["appimage"] if ...)` special case (AppImage is now native),
the `built_formats`/`tauri_bundles` deb-coupling for the thin builder, and the
block that discovers a deb and invokes the thin builder. `tauri build` receives
`appimage` directly in `--bundles`. After the Tauri build, call
`appimage_inventory.write_notices` on the extracted payload before the gate runs.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd scripts && python3 test_build_release.py -v && python3 test_check_release_bundle.py -v`
Expected: PASS both

- [ ] **Step 5: Verify the whole script suite and disk budget**

Run: `cd scripts && for t in test_*.py; do echo "== $t"; python3 "$t" 2>&1 | tail -3; done`
Expected: every module OK
Run: `cd /home/chen/githubprojects/fontpixel/fontcarver && python3 scripts/check_artifacts.py`
Expected: `PASS`

- [ ] **Step 6: Commit**

```bash
git add -A scripts/
git commit -m "feat(release): build the stock AppImage and drop the thin recipe"
```

---

### Task 7: Documentation

**Files:**
- Modify: `docs/licensing.md:133-135` (§5 Forbidden) and `:158` onward (§5.1)
- Modify: `THIRD_PARTY.md` §3.4
- Modify: `scripts/release_licenses/native/HOST-LIBRARIES.txt`
- Modify: `docs/releasing.md`, `docs/install.md`, `docs/STATUS.md`

**Interfaces:**
- Consumes: the rule as implemented in Tasks 1, 2 and 4. The documented rule must
  match `appimage_licenses.classify_labels`' behavior exactly, including that a
  GPL label requires a recorded ruling rather than an automatic refusal.
- Produces: nothing other tasks consume.

- [ ] **Step 1: Rewrite §5's dependency rule in two layers**

Replace `docs/licensing.md:133-135` so the forbidden list applies to
**application dependencies** (the Cargo and npm graphs) exactly as today, and add
the native-payload layer: LGPL permitted with the LGPL-2.1 §6 materials; GPL,
AGPL and SSPL forbidden; an unidentified license forbidden until identified; a
GPL-labelled package permitted only via a ruling recorded in
`appimage_licenses.RULINGS` with a reason. State that
`bundleMediaFramework` must stay disabled, and why.

- [ ] **Step 2: Rewrite §5.1 to state the real reason**

Per spec §3.1: bundling LGPL libraries does not change FontCarver's license,
because LGPL permits linking from differently licensed programs and the
application is aggregated with the runtime rather than linked to it. The
obligation is notice plus source availability. Keep the existing caution that
Debian copyright files describe source packages and are a review lead rather than
a per-binary ruling. Keep the rule that GTK/WebKit are never copied into the deb.

- [ ] **Step 3: Update THIRD_PARTY.md §3.4**

Keep the inventory numbers (177 shared objects, 126 packages, 73 copyleft labels,
10 shipped copyright documents; the 2026-10-02 rerun's 129/75/30/10). Add the
reading guidance from spec §3.3. Replace the thin-recipe paragraph with the
notice mechanism: `licenses/appimage/BUNDLED-LIBRARIES.txt` plus both LGPL texts,
generated by `appimage_inventory.py --notice-dir` and verified by
`appimage_policy.require_releasable_appimage`.

- [ ] **Step 4: Update the host-library notice and release docs**

In `HOST-LIBRARIES.txt`, drop the FUSE and squashfuse paragraph (nothing links
them now) and keep GTK/WebKit for the deb. The LGPL-2 and LGPL-2.1 texts and
their `HOST_LICENSE_HASHES` entries in `check_release_licenses.py` stay — they are
now also the AppImage's §6 documents. Update `docs/releasing.md`,
`docs/install.md` and `docs/STATUS.md` for the new Linux artifact set.

- [ ] **Step 5: Verify no stale references remain**

Run: `cd /home/chen/githubprojects/fontpixel/fontcarver && grep -rniI "thin.appimage\|build_thin_appimage\|host-libs-v1" docs/ scripts/ THIRD_PARTY.md | grep -v docs/acceptance/ | grep -v docs/superpowers/`
Expected: no output other than historical acceptance records

- [ ] **Step 6: Commit**

```bash
git add docs/ THIRD_PARTY.md scripts/release_licenses/
git commit -m "docs(licensing): permit LGPL in native payloads, forbid GPL and unidentified"
```

---

## Open item the plan cannot pre-size

Spec §10's first risk lands here: the **30 bundled packages with no
machine-readable license label** now block a release rather than being recorded as
unknown. Task 4's gate names them; resolving each one is manual work whose size is
unknown until the real list exists.

The first action after Task 6 is therefore to build one AppImage and run the gate
to produce that list. Only then can the remaining work be estimated. The options
per package are to identify its license by hand and add a ruling with a reason, or
to stop bundling it. Do not widen `classify_labels` to make them pass.
