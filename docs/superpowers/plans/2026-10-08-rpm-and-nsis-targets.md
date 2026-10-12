# rpm and nsis Bundle Targets Implementation Plan (Plan B)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `rpm` to the Linux release set and replace the two-artifact `msi` with a single multi-language `nsis` installer, with the payload inspector covering both.

**Architecture:** `scripts/check_release_bundle.py` hardcodes format knowledge in five places (`HOSTS`, `extracted()`, `platform_metadata()`, `inspect_payload()`, `signatures`). rpm is inspected exactly like deb once extracted, so it joins the deb branches rather than getting its own. nsis replaces msi wholesale: both are PE installers, but `msiexec /a` does not apply, so extraction uses `7z x`. Configuration and the target map change last, after the inspector can already verify what they produce.

**Tech Stack:** Python 3.12 stdlib only. Tests are `unittest` modules under `scripts/`, run as `python3 scripts/test_<name>.py` from the repo root.

**Spec:** `docs/superpowers/specs/2026-10-07-release-channels-design.md` (Plan B = spec §7 except install verification, which is Plan C)

## Global Constraints

- Python stdlib only; no new dependencies.
- Error style: raise `VerificationError` from `check_release_bundle.py`, `ValueError` from
  `build_release.py`, with a message naming the artifact or field at fault.
- `rpm` and `rpm2cpio` are **not installed on the development machine**; `cpio` and
  `7z` are. Unit tests for the rpm branch therefore mock `check_release_bundle.run`
  and must not require rpm tooling. End-to-end rpm verification needs `rpm`
  installed, so the release workflow gains it.
- Do not widen version rules while removing MSI's constraint: the numeric
  `major.minor.patch` rule stays, only its justification changes.
- Windows ships **one** installer after this plan. Do not leave msi behind as a
  second target.
- Run `python3 scripts/check_artifacts.py` before reporting each task complete.

## Review Focus

1. **A missing `rpm2cpio` must fail with a clear message, not a confusing `FileNotFoundError` from deep inside extraction.** The development machine lacks it, so this is the first thing a contributor will hit.
2. **`7z x` must not silently produce an empty payload.** 7-Zip exits 0 on some archives it only partially understands; an empty payload would otherwise reach the inspector and fail with a misleading "missing executable".
3. **An rpm whose `%{NAME}`/`%{VERSION}`/`%{ARCH}` query returns fewer than three lines must be rejected**, the same way the deb branch rejects missing `Package`/`Version`/`Architecture`.
4. **`validate_built_artifacts` must expect exactly one nsis artifact.** The old code expected two for msi via `msi_languages`; carrying that over would pass a build that produced only one of two expected files, or fail a correct single-artifact build.
5. **`platform_metadata` for rpm must require a desktop entry and icon**, as deb does. Falling through to the PE branch's `{'executable_format': 'PE'}` would silently skip Linux desktop-integration checks.

---

### Task 1: rpm support in the payload inspector

**Files:**
- Modify: `scripts/check_release_bundle.py` — `HOSTS:30`, `extracted():70`, `platform_metadata():208`, `inspect_payload():266`
- Test: `scripts/test_check_release_bundle.py`

**Interfaces:**
- Consumes: existing `run(command, cwd=None) -> bytes`, `VerificationError`.
- Produces: `'rpm'` accepted by `extracted`, `inspect_payload`, `platform_metadata`;
  `HOSTS['rpm'] == 'linux'`. Task 3 relies on `'rpm'` being a valid `--expected` value.

- [ ] **Step 1: Write the failing test**

```python
    def test_rpm_metadata_requires_name_version_and_architecture(self):
        with tempfile.TemporaryDirectory() as directory:
            artifact = Path(directory) / 'FontCarver-0.1.0-1.x86_64.rpm'
            artifact.write_bytes(b'\xed\xab\xee\xdb')
            scratch = Path(directory) / 'scratch'
            scratch.mkdir()
            with patch.object(verifier, 'run', return_value=b'FontCarver\n'):
                with self.assertRaisesRegex(verifier.VerificationError, 'NAME, VERSION'):
                    with verifier.extracted(artifact, 'rpm', scratch) as _:
                        pass

    def test_rpm_without_rpm2cpio_fails_with_a_named_tool(self):
        with tempfile.TemporaryDirectory() as directory:
            artifact = Path(directory) / 'FontCarver-0.1.0-1.x86_64.rpm'
            artifact.write_bytes(b'\xed\xab\xee\xdb')
            scratch = Path(directory) / 'scratch'
            scratch.mkdir()
            with patch.object(verifier, 'run', return_value=b'FontCarver\n0.1.0\nx86_64\n'):
                with patch.object(verifier.shutil, 'which', return_value=None):
                    with self.assertRaisesRegex(verifier.VerificationError, 'rpm2cpio'):
                        with verifier.extracted(artifact, 'rpm', scratch) as _:
                            pass

    def test_rpm_payload_is_inspected_like_a_deb(self):
        self.assertEqual(verifier.HOSTS['rpm'], 'linux')
        report = verifier.inspect_payload(self.root, 'rpm')
        self.assertEqual(report['executable']['paths'], ['usr/bin/fontcarver-desktop'])
        self.assertIn('host_library_notices', report)
        self.assertIn('desktop', report['platform_metadata'])
```

`self.root` is the Linux payload the existing `setUp` already builds for the deb
tests; reuse it unchanged. Add `import shutil` to `check_release_bundle.py` if it
is not already imported — the test patches `verifier.shutil.which`.

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 scripts/test_check_release_bundle.py -v`
Expected: FAIL — `KeyError: 'rpm'` from `HOSTS` and/or `AttributeError` for `verifier.shutil`

- [ ] **Step 3: Write minimal implementation**

In `HOSTS`:

```python
HOSTS = {'deb': 'linux', 'rpm': 'linux', 'appimage': 'linux', 'dmg': 'darwin', 'msi': 'win32'}
```

In `extracted()`, after the `deb` branch:

```python
    elif kind == 'rpm':
        fields = run(['rpm', '-qp', '--queryformat', '%{NAME}\\n%{VERSION}\\n%{ARCH}\\n',
                      str(artifact)]).decode().splitlines()
        if len(fields) < 3 or not all(field.strip() for field in fields[:3]):
            raise VerificationError('rpm lacks NAME, VERSION, or ARCH metadata')
        metadata = dict(zip(('Package', 'Version', 'Architecture'), fields[:3]))
        if shutil.which('rpm2cpio') is None or shutil.which('cpio') is None:
            raise VerificationError('rpm verification needs rpm2cpio and cpio on PATH')
        archive = run(['rpm2cpio', str(artifact)])
        result = subprocess.run(['cpio', '-idm', '--quiet'], input=archive, cwd=payload,
                                capture_output=True)
        if result.returncode:
            raise VerificationError(f'rpm payload extraction failed: {result.stderr.decode()[-500:]}')
        yield payload, metadata
```

In `platform_metadata()`, widen the Linux branch:

```python
    if kind in ('deb', 'rpm', 'appimage'):
```

In `inspect_payload()`, widen both Linux checks — the binaries filter and the host
library notices:

```python
    if kind in ('deb', 'rpm', 'appimage'):
        binaries = [path for path in binaries if path == root / 'usr/bin/fontcarver-desktop']
```

```python
    signatures = {'deb': (b'\x7fELF',), 'rpm': (b'\x7fELF',), 'appimage': (b'\x7fELF',),
                  'msi': (b'MZ',),
```

```python
    if kind in ('deb', 'rpm', 'appimage'):
        report['host_library_notices'] = host_library_notices(resource_root)
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 scripts/test_check_release_bundle.py -v`
Expected: PASS, existing tests plus the 3 new ones

- [ ] **Step 5: Commit**

```bash
git add scripts/check_release_bundle.py scripts/test_check_release_bundle.py
git commit -m "feat(release): inspect rpm payloads alongside deb"
```

---

### Task 2: nsis support, msi removal

**Files:**
- Modify: `scripts/check_release_bundle.py` — `HOSTS:30`, `extracted()` final branch, `inspect_payload():272` and `:280`, `:288`, `main()`'s `--expected` help
- Test: `scripts/test_check_release_bundle.py`

**Interfaces:**
- Consumes: Task 1's `HOSTS` shape.
- Produces: `HOSTS['nsis'] == 'win32'` with no `'msi'` key, and
  `SUFFIXES: dict[str, str]` mapping each kind to its artifact extension. Task 3
  relies on `'nsis'` being a valid `--expected` value, on `'msi'` being rejected,
  and on importing `SUFFIXES` for `artifact_snapshot`.

**Why `SUFFIXES` is needed:** `discover()` currently matches artifacts with
`path.suffix.lower() == f'.{kind}'`, and `build_release.artifact_snapshot` with
`path.suffix.lower().lstrip(".") in bundles`. That identity holds for deb, rpm, dmg
and appimage but **not for nsis**, whose artifact is `FontCarver_0.1.0_x64-setup.exe`
— suffix `.exe`. Without the map, every nsis build fails as "missing expected nsis
installer" and no snapshot entry is ever taken for it.

- [ ] **Step 1: Write the failing test**

```python
    def test_nsis_replaces_msi_as_the_windows_target(self):
        self.assertEqual(verifier.HOSTS['nsis'], 'win32')
        self.assertNotIn('msi', verifier.HOSTS)

    def test_nsis_artifacts_are_discovered_by_their_exe_suffix(self):
        self.assertEqual(verifier.SUFFIXES['nsis'], '.exe')
        self.assertEqual(verifier.SUFFIXES['rpm'], '.rpm')
        with tempfile.TemporaryDirectory() as directory:
            bundle = Path(directory)
            (bundle / 'nsis').mkdir()
            installer = bundle / 'nsis/FontCarver_0.1.0_x64-setup.exe'
            installer.write_bytes(b'MZ')
            self.assertEqual(verifier.discover(bundle, ['nsis']), [('nsis', installer)])

    def test_nsis_extraction_rejects_an_empty_payload(self):
        # 7z exits 0 on archives it only partly understands; an empty payload must
        # not reach the inspector as a misleading "missing executable".
        with tempfile.TemporaryDirectory() as directory:
            artifact = Path(directory) / 'FontCarver_0.1.0_x64-setup.exe'
            artifact.write_bytes(b'MZ')
            scratch = Path(directory) / 'scratch'
            scratch.mkdir()
            with patch.object(verifier, 'sys') as fake_sys:
                fake_sys.platform = 'win32'
                with patch.object(verifier, 'run', return_value=b''):
                    with self.assertRaisesRegex(verifier.VerificationError, 'extracted no files'):
                        with verifier.extracted(artifact, 'nsis', scratch) as _:
                            pass
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 scripts/test_check_release_bundle.py -v`
Expected: FAIL — `KeyError: 'nsis'`, and `'msi'` still present in `HOSTS`

- [ ] **Step 3: Write minimal implementation**

`HOSTS` drops `msi` and gains `nsis`, and a sibling map gives each kind its suffix:

```python
HOSTS = {'deb': 'linux', 'rpm': 'linux', 'appimage': 'linux', 'dmg': 'darwin',
         'nsis': 'win32'}
# An artifact's extension is not always its bundle name: Tauri's nsis output is
# `<name>_<version>_<arch>-setup.exe`.
SUFFIXES = {'deb': '.deb', 'rpm': '.rpm', 'appimage': '.appimage', 'dmg': '.dmg',
            'nsis': '.exe'}
```

and `discover()` matches on it instead of on the kind name:

```python
        matches = sorted(path for path in bundle.rglob('*')
                         if path.is_file() and path.suffix.lower() == SUFFIXES[kind])
```

Replace the trailing `else:` msi branch in `extracted()`:

```python
    else:
        run(['7z', 'x', f'-o{payload}', '-y', str(artifact)])
        if not any(payload.rglob('*')):
            raise VerificationError('nsis installer extracted no files')
        yield payload, {'extraction': '7z extraction'}
```

In `inspect_payload()`, replace every `msi` mention with `nsis`:

```python
    executable_name = 'fontcarver-desktop.exe' if kind == 'nsis' else 'fontcarver-desktop'
```

```python
                  'nsis': (b'MZ',),
```

```python
        if any(header.startswith(signature) for signature in signatures[kind]) and (
                kind == 'nsis' or os.access(path, os.X_OK)):
```

In `main()`:

```python
    parser.add_argument('--expected', required=True, help='comma-separated deb,rpm,appimage,dmg,nsis')
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 scripts/test_check_release_bundle.py -v`
Expected: PASS, all tests

- [ ] **Step 5: Commit**

```bash
git add scripts/check_release_bundle.py scripts/test_check_release_bundle.py
git commit -m "feat(release): verify the nsis installer and drop msi"
```

---

### Task 3: Target map, bundle configuration and CI

**Files:**
- Modify: `scripts/build_release.py` — `native_bundles():28`, `validate_versions():37`, `validate_built_artifacts():70`, the wix language lookup
- Modify: `src-tauri/tauri.conf.json` — `bundle.linux`, `bundle.windows`
- Modify: `.github/workflows/release.yml` — Linux prerequisites
- Test: `scripts/test_build_release.py`

**Interfaces:**
- Consumes: Tasks 1 and 2 — `'rpm'` and `'nsis'` must already be valid `--expected`
  values, since `build_release.py` passes `--expected ",".join(bundles)` to
  `check_release_bundle.py`.
- Produces: nothing later tasks consume; this is the last task.

- [ ] **Step 1: Write the failing test**

```python
    def test_native_targets_cover_rpm_and_nsis(self):
        self.assertEqual(native_bundles("Linux"), ["deb", "rpm", "appimage"])
        self.assertEqual(native_bundles("Darwin"), ["dmg"])
        self.assertEqual(native_bundles("Windows"), ["nsis"])

    def test_one_artifact_is_expected_per_format(self):
        import inspect
        # The msi_languages parameter defaulted to 2; a build is correct with one
        # artifact per format now, so the parameter must be gone, not just unused.
        self.assertNotIn("msi_languages",
                         inspect.signature(validate_built_artifacts).parameters)
        with tempfile.TemporaryDirectory() as directory:
            bundle = Path(directory)
            for kind in ("nsis", "rpm"):
                (bundle / kind).mkdir()
            (bundle / "nsis/FontCarver_0.1.0_x64-setup.exe").write_bytes(b"MZ")
            (bundle / "rpm/FontCarver-0.1.0-1.x86_64.rpm").write_bytes(b"\xed\xab\xee\xdb")
            before = {}
            validate_built_artifacts(bundle, ["nsis", "rpm"], before)
            (bundle / "nsis/FontCarver_0.1.0_x86-setup.exe").write_bytes(b"MZ")
            with self.assertRaisesRegex(ValueError, "nsis"):
                validate_built_artifacts(bundle, ["nsis", "rpm"], before)

    def test_bundle_configuration_declares_rpm_and_nsis(self):
        import json
        config = json.loads((Path(build_release.__file__).resolve().parents[1]
                             / "src-tauri/tauri.conf.json").read_text())
        self.assertIn("rpm", config["bundle"]["linux"])
        self.assertIn("nsis", config["bundle"]["windows"])
        self.assertNotIn("wix", config["bundle"]["windows"])
        self.assertEqual(config["bundle"]["windows"]["nsis"]["languages"],
                         ["English", "SimpChinese"])
```

`validate_built_artifacts`'s fourth parameter is removed in Step 3, so the call
above passes three arguments.

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 scripts/test_build_release.py -v`
Expected: FAIL — Linux is `["deb", "appimage"]`, Windows is `["msi"]`, `wix` still present

- [ ] **Step 3: Write minimal implementation**

`scripts/build_release.py`:

```python
    bundles = {"Linux": ["deb", "rpm", "appimage"], "Darwin": ["dmg"], "Windows": ["nsis"]}
```

Drop the `msi_languages` parameter and its use:

```python
def validate_built_artifacts(bundle: Path, bundles: list[str], before: dict) -> None:
```

```python
        expected = 1
```

Delete the two lines that read the WiX language list and compute `language_count`,
and pass three arguments at both `validate_built_artifacts` call sites.

`artifact_snapshot` must stop deriving the extension from the bundle name, for the
same reason `discover()` did — import Task 2's map and use it:

```python
from check_release_bundle import SUFFIXES, VerificationError, discover
```

```python
def artifact_snapshot(bundle: Path, bundles: list[str]) -> dict[str, tuple[int, int, int]]:
    """Remember existing candidates so a successful build cannot accept stale files."""
    wanted = {SUFFIXES[kind] for kind in bundles}
    return {str(path): (path.stat().st_mtime_ns, path.stat().st_ctime_ns, path.stat().st_size)
            for path in bundle.rglob("*") if path.is_file()
            and path.suffix.lower() in wanted}
```

In `validate_versions`, keep the numeric rule and restate why it exists:

```python
    """Reject mismatched package versions and versions a native installer cannot encode."""
```

and change the two messages from "accepted by MSI" / "Windows MSI version limits"
to "accepted by native installers" / "native installer version limits".

`src-tauri/tauri.conf.json` — add `rpm` beside `deb`, mirroring its copyright file:

```json
    "rpm": {
      "files": {
        "/usr/share/doc/font-carver/copyright": "packaging/linux/copyright"
      }
    }
```

and replace the whole `wix` object with:

```json
    "nsis": {
      "languages": ["English", "SimpChinese"],
      "displayLanguageSelector": true
    }
```

`.github/workflows/release.yml` — the Linux prerequisites gain `rpm`, which the
inspector needs to query metadata:

```
            desktop-file-utils gcc binutils pkg-config squashfs-tools rpm
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 scripts/test_build_release.py -v && python3 scripts/test_check_release_bundle.py -v`
Expected: PASS both

- [ ] **Step 5: Verify the whole suite, the workflow and the disk budget**

Run: `for t in scripts/test_*.py; do echo "== $t"; python3 "$t" 2>&1 | tail -2; done`
Expected: every module OK
Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml')); print('yaml ok')"`
Expected: `yaml ok`
Run: `python3 scripts/check_artifacts.py`
Expected: `PASS`

- [ ] **Step 6: Commit**

```bash
git add scripts/build_release.py scripts/test_build_release.py src-tauri/tauri.conf.json .github/workflows/release.yml
git commit -m "feat(release): ship rpm and a single nsis installer"
```

---

## Documentation

Folded into Task 3's commit rather than given its own task, because the artifact
set is one fact stated in several places: `docs/releasing.md` (the helper's target
list and the per-format verification notes), `docs/install.md` (the download table
and per-format install steps, including that rpm uses the same host-library model
as deb), and `THIRD_PARTY.md` §3.4 only if it names the artifact set. `docs/STATUS.md`
carries the user's uncommitted work and is left alone, as in Plan A.

## Out of scope

- **rpm install verification** is Plan C. This plan makes the artifact and inspects
  its payload; it does not install it. `docs/STATUS.md`'s release standard is not
  met for rpm until Plan C lands, and that gap must be stated in `docs/releasing.md`
  rather than left implicit.
- **`7z` reliability on NSIS installers** is the spec's known risk. This plan adds
  the empty-payload guard (Review Focus 2) but cannot confirm real behaviour without
  a Windows build; that confirmation belongs to the real-build work.
