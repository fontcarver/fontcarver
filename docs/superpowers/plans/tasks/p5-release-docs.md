# P5 release documentation

Owner: release documentation subagent, delegated by the P5 release owner.

## Scope

Write `docs/install.md` for native package selection, installation, removal and
retained user data, and `docs/releasing.md` for contributors building and
verifying native packages. Keep instructions aligned with the release owner's
Tauri configuration, build helper, verifier and artifact-only CI workflow.

No changes to package implementation, README, shared status/audit files, or
system installation. No publishing, signing, notarization, or native/global
desktop input automation. Linux observations do not accept macOS or Windows.

## Interfaces

- Repository version and package names come from Cargo, npm and Tauri metadata.
- `scripts/build_release.py` owns the native build entry point; its final CLI
  and the packaging verifier must be checked before documenting commands.
- Package outputs target Linux deb/AppImage, macOS dmg and Windows msi.
- `FC_CONFIG_DIR` and the actual config implementation determine user data
  instructions; never infer platform-specific directories from convention.

## Acceptance

- Cross-check every documented local path, tool flag and artifact name against
  source/configuration or the implementation owner's actual build evidence.
- Verify Markdown links, command examples and whitespace in the two docs.
- State signing/notarization and unrun native-platform acceptance explicitly.
- Check `python3 scripts/check_artifacts.py` at start and completion. This
  documentation task does not need a Rust or frontend rebuild.
- Record final documentation checks and boundaries here; the release owner
  maintains combined packaging acceptance.

## Evidence

Task-start budget check: passed, 5.23 GiB build artifacts, 0.00 GiB temporary
artifacts and 670.46 GiB workspace free.

Documentation written and reviewed on 2026-09-30. An independent source audit
confirmed platform config/recovery locations and separate WebView localStorage.
A second reviewer checked installation claims and identified the Python 3.11+
requirement and configured macOS 11 minimum; both are documented. Commands and
the native workflow matrix were checked against the release owner's final
interfaces and files.

Verification:

```text
python3 scripts/build_release.py --help
PASS: documented --bundles and --min-free-gib options exist.

python3 scripts/check_release_bundle.py --help
PASS: documented --bundle-dir, --expected and --output options exist.

Local Markdown link / shell syntax / whitespace check
PASS: 6 local links resolve; 6 shell examples parse; no trailing whitespace

pnpm --dir app exec prettier --check ../docs/install.md ../docs/releasing.md ../docs/superpowers/plans/tasks/p5-release-docs.md
Checking formatting...
All matched files use Prettier code style!

python3 scripts/check_artifacts.py
build: 6.08 GiB, 13,160 files
temporary: 0.03 GiB, 3,029 files
workspace free: 669.23 GiB
PASS
```

This documentation task ran no native builds, installations, signing,
notarization, desktop input automation or publication. It created no temporary
files or processes requiring cleanup. Native build evidence belongs to the
release owner's combined acceptance report; macOS/Windows remain unaccepted
until actual native checks are recorded.
