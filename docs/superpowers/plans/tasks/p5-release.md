# P5 release packaging boundary

Owner: release agent; resumed 2026-09-30.

## Scope and files

Provide reproducible native installer build commands, explicit Tauri bundle
metadata/resources, and a read-only-permission GitHub Actions matrix producing
Linux deb/AppImage, macOS dmg, and Windows msi artifacts. Add package-content
verification and SHA-256 manifests. Build and inspect Linux artifacts locally
after the parent grants the serialized broad-build slot. Do not publish, tag,
push, install systemwide on the user's host, or automate native desktop input.

Owned files: `.github/workflows/release.yml`, `src-tauri/tauri.conf.json` bundle
settings, `scripts/build_release.py`, `scripts/check_release_bundle.py`, focused
script tests, and `docs/p5-release-acceptance.md`. Delegated release documentation
and dependency notices have separate task records. Other Tauri runtime/window
changes remain owned by the shell/workspace agent.

## Acceptance

- Native build wrapper works without Unix-only npm environment assignments;
  checks version alignment, locked dependencies, notices and artifact budgets.
- Packages include the application executable, icons/desktop metadata when
  applicable, both project licenses, third-party notices and user instructions.
- Linux deb/AppImage contents and checksums are independently inspected; package
  extraction stays inside automatically cleaned task scratch directories.
- CI specifies native builds and installation checks on all three systems,
  uploads installers plus evidence, and has no release publishing permission.
- Existing tests and final artifact-budget checks pass. Record actual Linux
  results and explicitly distinguish unexecuted macOS/Windows CI definitions
  from native-platform acceptance. Audit rows 9 and 62 remain pending until
  their complete interactive/native acceptance can be observed.
