# Wave 3 native workflow and startup checks

Date: 2026-10-02. Task: [p5f-w3-native](superpowers/plans/tasks/p5f-w3-native.md).
Rows **9** and **62** remain **Partial**: this Linux checkout cannot supply
native macOS/Windows execution evidence. This task improves the checks those
runners will execute and records their actual scope.

## Changes

- Native workflows now exercise the six existing platform modifier, shortcut,
  keymap and window-routing test files on every target. Their simulated DOM
  assertions are not native desktop input evidence.
- Installed Debian/AppImage executables must identify the exact expected
  version and then exit with FontCarver's precise no-display error. An arbitrary
  nonzero exit, missing shared library, crash or timeout no longer passes.
- Installed macOS and Windows executables must identify their version and
  survive a bounded startup interval. The JSON report explicitly states that
  UI readiness, interactive parity and signing are unverified.
- `smoke_release_native.py` isolates its application configuration, captures
  output and stops owned processes. On Windows a wrapper joins a fresh
  kill-on-close Job Object **before** starting the application. Assignment
  failure aborts before launch. This prevents children escaping cleanup when
  their parent exits early; no taskkill process-tree snapshot is required.
  [Microsoft documents Job Object inheritance and kill-on-close behavior](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).
- Debian/macOS cleanup uses traps, and Windows uninstall uses `finally`.
  Success and failure artifacts include the startup JSON reports. All release
  scripts trigger native CI; Linux validates every script test suite, with
  portable release tests on macOS/Windows.
- Main CI explicitly builds the CLI used by hinting E2E fixtures and explains
  per-test kernel isolation correctly. CI installs `ttfautohint` as an external
  system test prerequisite and explicitly runs the two ignored real-hinter
  tests; no executable is linked or included in FontCarver packages.
- Both workflows place test temporary files under their own scratch directory.

## Validation

Initial regression run before implementation failed with
`ModuleNotFoundError: No module named 'smoke_release_native'`.

Final focused commands:

```text
TMPDIR="$PWD/.scratch/p5f-w3-native/tmp" python3 -m unittest discover -s scripts -p test_smoke_release_native.py
Ran 9 tests in 0.609s
OK

TMPDIR="$PWD/.scratch/p5f-w3-native/tmp" python3 -m unittest discover -s scripts -p test_release_windows_job.py
Ran 4 tests in 0.001s
OK (skipped=1)
```

The skipped test actually starts a Windows Job Object and checks that an
orphaned child cannot survive its parent. It is scheduled in Windows native CI;
Linux's mocked API checks do not stand in for that result.

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/shell/shortcuts.test.ts src/shell/keymap.test.ts \
  src/shell/keymapIntegration.test.ts src/shell/keymapPresets.test.ts \
  src/editor/modifiers.test.ts src/transport/windows.test.ts
Test Files  6 passed (6)
Tests       60 passed (60)
Duration    5.30s
```

Both workflow files parse as YAML, all Bash steps pass `bash -n`, and
`actionlint .github/workflows/ci.yml .github/workflows/release.yml` exits 0 with
no output. The temporary checker came from the
[official actionlint v1.7.12 release](https://github.com/rhysd/actionlint/releases/tag/v1.7.12);
its archive SHA-256 was verified as
`8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8`.
`git diff --check` is clean. These are static workflow checks, not a hosted run.

## Review round

The independent release reviewer identified Windows child cleanup after an
early parent exit and incomplete PR path/test coverage. The Job Object wrapper,
fail-closed regression tests, actual Windows orphan test, `scripts/**` trigger
and full Linux helper suite address those findings. Linux headless timeout
handling and successful-but-premature GUI exits also have regression coverage.

## External conditions and publication

- Host: Linux x86_64. No native macOS or Windows host is available. Installed
  Wine is not native Windows evidence and was not used for acceptance.
- Read-only `gh repo view` probes of both the literal `fontvarver/fontcarver`
  and configured origin `fontcarver/fontcarver` return “Could not resolve to a
  Repository”. No repository was created, pushed or dispatched.
- The user explicitly requested **no publication** and confirmed signing
  credentials are not configured. No signing secret was read, fabricated or
  configured. The workflow remains an unsigned installer-candidate workflow;
  it does not create a release or deploy the website.
- Actual rebuilt Linux artifact installation/runtime results belong to the
  packaging acceptance. This record claims neither a native GUI render nor
  complete cross-platform parity from process survival.

No new application dependencies or i18n keys. No FontForge source used.

## Resource completion

The final checker passed: build **15.73 GiB / 19,219 files**, temporary
**0.01 GiB / 573 files**, free **606.24 GiB**. The 10 GiB warning remains; the
20 GiB ceiling is respected. This task's 8.1 MiB scratch/tool download and its
Python bytecode files were removed; no owned test, browser or app process
remains. Other agents' artifacts were left intact.

## Thin AppImage integration follow-up

The independent release reviewer found that the new thin runtime's reviewed
squashfuse 0.5.0 headers were unavailable in the former Ubuntu 22.04 CI image:
[Jammy ships 0.1.103-3](https://packages.ubuntu.com/source/jammy/squashfuse),
whereas [Noble ships 0.5.0-2build1](https://packages.ubuntu.com/source/noble/squashfuse).
The native Linux matrix now uses Ubuntu **24.04** and installs `gcc`,
`binutils`, `pkg-config`, `squashfs-tools`, `libsquashfuse-dev` and
`libfuse3-dev`. A pre-build check requires `pkg-config --modversion squashfuse`
to report `0.5.0`; the obsolete FUSE 2 prerequisite was removed. Packaging owns
the matching supported-baseline documentation.

Both workflows again pass actionlint v1.7.12, YAML parsing and Bash syntax
checks; `git diff --check` passes. No hosted run was performed. Follow-up scratch
was removed. Resource check: **15.88 GiB / 19,638 build files**, **0.01 GiB /
589 temporary files**, **606.06 GiB free**, PASS with the existing build warning.
