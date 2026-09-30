# Stopped checkpoint and future work

The latest user instruction is to finish the active batch, then stop and rest.
Do not start another batch. Branch: `p5-color-authoring`; changes remain local and
uncommitted, with no push or main merge. Native/global desktop input automation
remains prohibited.

The [integration checkpoint](p5-resume-integration.md) records final verification.
The [feature audit](p5-feature-audit.md) remains the completion gate; the following
items are documented backlog, not instructions to continue now.

## Completed in the frozen batch

- Variable pen parameters and previews, bake/undo and native save/reopen.
- Cross-document compare/paste/merge; local scripts, Python RPC and bundled plugins.
- Interchange format subsets, export profiles/preflight and shared CLI options.
- Workspaces/detached panels, shortcut editing/help, proofing and glyph workflows.
- Construction/image/optical/bitmap/CJK/feature editing subsets and hint preservation.
- Independent Chinese translation, licensing/release definitions and audit reconciliation.

See owner reports linked from the integration checkpoint for exact boundaries.
No broad requirement is complete merely because its implementation or tests exist.

## Remaining acceptance and product boundaries

| Area                | Remaining scope                                                                                                                                                                                             |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Audit               | Reconcile 13 historical acceptance candidates; verify every broad row's missing qualifiers and actual user workflows. The historical 60% estimate is not recalculated.                                      |
| Hinting             | Authored variable-font hints, imported flex binding, dotsection and general counter semantics; CFF2 private variation data. CFF1 multi-dictionary preservation has independent decoding/rendering evidence. |
| Geometry/editor     | Independent variable-pen export coverage beyond the current tests; radius drag/crossing construction, open-corner and remaining text/pixel tool behavior; broader problem detection.                        |
| Construction/images | Affine construction is bounded; no general deformation/trim. Linked-node inspectors lack canvas drag magnets/global guide behavior. PNG tracing remains polygonal rather than photographic curve fitting.   |
| Variable/CJK        | No cross-document Family hierarchy, drag-swap compatibility repair, compound bracket conditions or main-canvas smart-component handles. Several browser workflows remain unverified.                        |
| Glyph/features      | Canvas2D grid, bounded language/name databases and filter vocabulary; nine bounded visual lookup types and a bounded token language, not arbitrary FEA/context graphs or an interpolated numeric manager.   |
| Color/proofing      | Static palette-zero previews only; axis-dependent paints and color proof export remain unsupported.                                                                                                         |
| Formats/export      | Supported static interchange subsets and raw-table TTX, not general semantic TTX XML; broader interoperability and actual export-dialog workflows require additional acceptance.                            |
| Automation          | Built-in plugin index only; Python requires a local interpreter. An already dispatched synchronous kernel call is not interruptible. Native Tauri execution remains unverified.                             |
| Release             | Real Linux deb/AppImage and macOS/Windows packages/installations remain unbuilt/unverified; CI definitions and helper tests are not package acceptance. Website deployment remains pending.                 |
| UI acceptance       | Component tests and selected headless workflows pass; they do not establish native-window, all-platform or every-screen acceptance.                                                                         |

Before a future build, run `python3 scripts/check_artifacts.py`. Retained shared
build dependencies are subject to the 20 GiB ceiling and 10 GiB warning in
`CONVENTIONS.md`; clean the development profile if another build would exceed the
ceiling. Do not delete shared stores, source/fonts, Git data or another task's files.
