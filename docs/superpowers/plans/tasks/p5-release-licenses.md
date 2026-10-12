# P5 release license gate and notices

Scope: add a standard-library-only release license audit and notice generator in
`scripts/check_release_licenses.py`, with focused tests in
`scripts/test_check_release_licenses.py`. Read the locked Cargo graph without
building and the installed pnpm graph. Apply the existing policy in
`docs/licensing.md`, including exact package/version rulings in `THIRD_PARTY.md`;
missing metadata and newly introduced exceptions fail closed.

Interface: `python3 scripts/check_release_licenses.py` audits project and
dependency metadata. `--output-dir PATH` additionally writes a distributable
notice directory containing project license texts, third-party attribution,
dependency provenance and upstream notice files. Notice scope conservatively
includes the full Cargo graph and production npm packages; metadata policy covers
both full graphs. Output must not conceal missing upstream notices in that scope.
Parent release task owns workflow integration and packaging.

Acceptance: unit tests cover SPDX boolean semantics, malformed/unknown licenses,
version-bound exception rulings, absent metadata/notices and complete output.
Run the checker against the installed locked graph; report any actual blocker.
No Rust builds, new dependencies, policy changes or legal rulings are in scope.

Resource checks: run `python3 scripts/check_artifacts.py` at task start and end;
temporary inventories live only under `.scratch/release-licenses/` and are removed
at completion. Parent owns broad repository checks.

## Result

- Ten focused Python tests pass. Live offline locked audit passes for 601 Cargo
  and 277 npm versions. Notice generation covers 601 Cargo and 29 production npm
  versions; generated files are SHA-256 verified.
- Exact-version supplemental catalogs preserve upstream notice bytes and pinned
  source URLs when published archives omit license files. No policy change,
  dependency addition, FontForge adaptation or `THIRD_PARTY.md` edit was made.
- Upstream caveats remain recorded in `scripts/release_licenses/cargo/README.md`:
  Apple SDK derivation, absent separate WebView2 SDK loader redistribution notice,
  and incomplete historical publication provenance for difflib/WinAPI.
- This task checks source-package metadata and retained notice documents. It does
  not inspect native libraries or bundler runtimes in actual artifacts (for
  example GTK/WebKit libraries collected into AppImages). No installers or
  AppImages were built here; their full distribution licensing remains outside
  this evidence and requires artifact inspection.
