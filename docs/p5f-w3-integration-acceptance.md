# Wave 3 integrated local acceptance

Date: 2026-10-02. Branch: `p5-color-authoring`. Implementation commits:
`979bcca` (real hinter), `f5051d2` (editor/browser repairs), `6e35f70` (release tooling).

The [112-row audit](p5-feature-audit.md) retains **110 Complete / 2 Partial**,
with independent [first-half](p5f-w3-audit-a.md) and
[second-half](p5f-w3-audit-b.md) reviews. Ordinary defects found during review
were fixed. [TODO](TODO.md) lists the accepted feature boundaries and remaining
delivery gates. The original separate translation checkpoint had 3,943 keys;
subsequent UX/discovery deltas leave **3,945 matching English/Chinese keys**.

## Original wave 3 integrated verification

Commands ran with default space-saving profiles, a shared Cargo lock and an
absolute task-local TMPDIR. [Output excerpts](p5f-w3-integration-validation.txt):

| Command                                                    | Final result                                                                            |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `cargo fmt --all --check`                                  | Pass                                                                                    |
| `cargo clippy --workspace --all-targets -- -D warnings`    | Pass                                                                                    |
| `cargo test --workspace --no-fail-fast`                    | 2,093 passed; 4 explicitly ignored; 279 targets                                         |
| Release `acceptance_workflow` and `edit_performance` tests | 2 passed; edit maximum 397.654 µs                                                       |
| Real external hinter acceptance                            | [Passed independently](p5f-w3-hinter-acceptance.md); covers the other two ignored tests |
| `pnpm check`, `pnpm lint`, `prettier --check .`            | Pass                                                                                    |
| `vitest run --maxWorkers=4 --minWorkers=1`                 | 2,836 tests / 342 files passed                                                          |
| `pnpm build`                                               | Pass; existing chunk-size advisory remains                                              |
| Default Playwright, zero retries, external CJK enabled     | [100/100 passed](p5f-w3-e2e-acceptance.md), 5.2 minutes                                 |
| Python release/helper tests                                | 89 passed; one Windows-only test skipped                                                |

Earlier runs remain failures/interrupted runs: the first frontend suite exposed
two stale cancellation assertions, then passed after correction; initial Clippy
found a corrected collapsible conditional. The first full E2E run was 99/100
with a 10.8 ms timing outlier; three focused repeats and the final complete run
passed the unchanged 8 ms limit. Its cause remains unproven.

## Subsequent UX integration

The user then requested actual editor use and input/layout improvement.
[The experience review](p5ux-experience-acceptance.md) records the reproduced
problems and before/after evidence. Source commits are `77c6282` (shortcuts),
`5d2c658` (outline gesture lifecycle), `4a8b36e` (bitmap gestures) and `4e0fc4f`
(compact layout, preview controls, wording and browser coverage).

Later source adds guarded bitmap entry focus (`ed9089d`), subpixel pointer-test
comparison (`a0496e8`), an explicit editor-ready test barrier (`7e32289`), the
user's one additional small [welcome-discovery task](p5ux-start-discovery-browser-acceptance.md)
(`7b9980c`) and command-search focus under development StrictMode (`f6f6cf9`).
The final frozen application/tooling tree is **`f6f6cf9`**.

Final frontend verification: **2,914 tests in 348 files passed**, type checking,
ESLint, Prettier and production build passed. The first full UX run had three
GlyphView failures caused by synthetic drag events missing pressed-button state
and an unresolved mock cancellation after failure; fixture repair retained the
assertions and the next run passed 2,897 tests. A later 2,906-test run exposed one
TextView test reading its controller before registration. An explicit readiness
barrier preserved that assertion; the final 2,914-test run passes.
[Retained output](acceptance/p5ux/validation.txt). Python helpers report
**97 passed / 1 Windows-only skipped** after the bundle-marker regression tests.
Rust code is unchanged since the full gate above, whose evidence remains valid.
GPT-6.1 Sol independently reviewed both locale deltas: **3,945 matching keys**.

Final [browser regression](p5ux-browser-acceptance.md): **105/105 passed** in
6.5 minutes, with zero failures, skips or retries and the actual external CJK
font enabled. Performance maximum **6.5 ms** passes the unchanged 8 ms limit.
The earlier UX round remains recorded as 102 passed / 2 failed / 1 not executed:
one pointer assertion needed subpixel tolerance (0.01 font unit), while the other
exposed the real bitmap entry-focus defect. Both focused workflows and the final
complete run passed after their respective fixes.

## Linux installers and resource closure

The final deb and thin AppImage were rebuilt from **`f6f6cf9`** and passed
[packaging acceptance](p5f-w3-packaging-acceptance.md),
[runtime acceptance](p5f-w3-thin-appimage-acceptance.md) and
[independent review](p5f-w3-review-release.md). The
[manifest](acceptance/p5f-w3-packaging/release-manifest.json) binds **2,008 inputs**
to source fingerprint
`2fe41daf9aa27d48230fb3ac970ab119867c49245f2dcdff47017c60c93a6624`.

| Final local artifact              |      Bytes | SHA-256                                                            |
| --------------------------------- | ---------: | ------------------------------------------------------------------ |
| `FontCarver_0.1.0_amd64.deb`      | 24,235,798 | `e01096f3abd387e5758dc177cfe5a85e4ec5030613d1f3f1c890aac789e429de` |
| `FontCarver_0.1.0_amd64.AppImage` | 23,683,640 | `3a9029b1aa83d39d78365b18202b39aa5a035f640083dfb4e53151c205793536` |

Real isolated APT install/reinstall/remove and user-data retention pass against
that exact deb. Both installed-binary checks pass. AppImage FUSE and
extract-and-run, version and expected no-display behavior pass; its process also
survives 15 seconds in an isolated virtual display without input automation.
This establishes graphical-process startup, not interactive desktop workflows.

Independent extraction verifies 630 dependency packages / 1,126 notice files,
nine runtime/source/license pins and zero bundled shared libraries. The main
executable differs from the actual deb only at the expected three-byte `DEB` to
`APP` marker; all 1,144 other deb files match. The thin image requires documented
compatible host libraries. Clean-OS dependency resolution and cross-version
upgrades remain unverified.

The first `6e35f70` round retained a DEB marker in the thin image and is
[archived as superseded](acceptance/p5f-w3-packaging/round-1-6e35f70/README.md).
The builder and fail-closed gate were corrected in `29c97d2`; the final checks
above include that correction and all subsequent UX/welcome changes.

An initial workspace build was interrupted when duplicate caches reached
28.31 GiB. Cleaning the dev profile restored the budget before the passing run.
Before native packaging, another planned dev cleanup removed 14.7 GiB:
**2.54 GiB build / 0.04 GiB temporary / 619.25 GiB free, PASS**.

After final frontend, browser, package and independent-review acceptance, owned
browser/server processes and task scratch data were removed. Final resource
and documentation closure are recorded in [STATUS](STATUS.md); final candidate
binaries remain under `target/release/bundle/` and are not committed.

The [independent final records review](p5ux-final-records-review.md) passed.
Source fingerprint and all 112 row IDs/counts were rechecked after documentation
updates. Raw dpkg logs and linker maps retain their generated trailing spaces;
the maps remain byte-identical to their recorded SHA-256 values. The staged
whitespace check excludes only those four raw evidence files; other changes
pass unchanged.

Publication, signing and native macOS/Windows execution are excluded by the
user's current scope and are not claimed complete.
