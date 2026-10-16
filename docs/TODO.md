# Remaining work

Updated: 2026-10-04. Branch: `p5-color-authoring`; local work only, no push,
`main` merge, signing or publication.

**All authorized local closing work and the selected UI follow-up tasks
are complete. Publication and macOS/Windows validation remain outside this batch.**

## With the next push of `main`

Local `main` (2026-10-16) has every branch merged and the website removed: the
site now lives in its own repository, fontcarver/fontcarver.com. Two remote
branches are fully merged into local `main` but still exist on GitHub. Delete
them in the same push, so nothing on GitHub points at work that is not on
`main`:

```sh
git push origin main :p5-color-authoring :release-publish
```

- [ ] Push `main` and delete `origin/p5-color-authoring` and `origin/release-publish`.

## Latest completed follow-up

- [x] Complete [actual editor experience](p5ux-experience-acceptance.md): laptop
      canvas/tool accessibility, progressive disclosure, safe outline/bitmap gesture
      cancellation and coherent keyboard/menu behavior.
- [x] Verify repaired workflows: **2,914 frontend tests / 348 files**, all
      check/lint/format/build gates, and **105/105 browser tests** with no retries
      or skips ([browser acceptance](p5ux-browser-acceptance.md)).
- [x] Complete the one additional small task explicitly requested by the user:
      [welcome-screen discovery](p5ux-start-discovery-browser-acceptance.md),
      linking existing getting-started help, command search and shortcut settings;
      fix command-search focus under StrictMode and verify both languages.
- [x] Rebuild final deb/thin AppImage from **`f6f6cf9`**, including the corrected
      bundle-type marker; repeat actual candidate inspection/install/startup and
      independent review.
- [x] Update final records and clean owned test environments; retain local
      commits, reports and final candidates.
- [x] Add an [illustrated welcome guide](user/context-help.md#start-from-the-welcome-screen)
      using accepted screenshots, the three entry labels in both languages and
      a command-search example (the user's separate documentation task).
- [x] Check welcome and command palette at actual 200% browser zoom in both
      languages/themes; fix long palette descriptions hiding command names and
      add four reflow regressions ([acceptance](p5ux-zoom-acceptance.md)).
- [x] Fix the English Help menu overflowing at 200%: keep all titles and popups
      inside the window, allow wrapping at narrower widths and preserve mouse/
      keyboard access. Four new menu cases plus the four welcome/palette cases
      pass ([acceptance](p5ux-menu-zoom-acceptance.md)).
- [x] Check Preferences and Keyboard Shortcuts at actual 200% zoom in both
      languages/themes. Wrap overflowing nudge fields; verify all preferences
      categories, list scrolling, conflict messages, footer controls and keyboard
      focus. Four new browser regressions pass
      ([acceptance](p5ux-settings-zoom-acceptance.md)).
- [x] Expand the 200% check to the glyph editor, sidebar/Inspector, Font Info,
      metadata editing and Export. Preserve canvas width with temporary narrow
      panels; wrap Font Info navigation; contain status and node-info scrolling;
      preserve keyboard focus after metadata Apply. English/Chinese × light/dark
      pass actual 200% checks at two window widths, including editing, undo and
      real TTF export. All **23 targeted browser cases** and **2,917 frontend
      tests** pass ([acceptance](p5ux-workspace-zoom-acceptance.md)).
- [x] Run the [latest complete E2E suite](p5ux-latest-e2e-acceptance.md), including
      all recent zoom changes: **125/125 pass**, zero failures, skips or retries
      in the final run. Correct two gesture-test setup/synchronization gaps found
      in earlier complete runs. The 300-point drag gate passes at **1.4 ms P95 /
      2.6 ms maximum**, below the unchanged 8 ms limit; the real 65,535-glyph CJK
      case also runs and passes.

The result of this batch belongs in [integrated acceptance](p5f-w3-integration-acceptance.md).
The [112-row gate](p5-feature-audit.md) retains **110 Complete / 2 Partial**;
Complete means the bounded behavior described by the linked acceptance evidence.

## Local installer verification

The actual integrated **`f6f6cf9`** candidates were rebuilt and passed the strict
[package verifier](p5f-w3-packaging-acceptance.md). The deb passed real isolated
APT install, same-version reinstall and removal, including installed-binary
checks, desktop-cache triggers and retention of user documents/preferences/
recovery files. The host package installation was unchanged.

The [thin AppImage](p5f-w3-thin-appimage-acceptance.md) passed real FUSE and
extract-and-run startup checks. Its manifest records zero bundled shared
libraries, all nine runtime/source/license pins and matching host-library
notices. The old default Tauri AppImage is rejected by the gate. This thin image
requires compatible host GTK/WebKit/FUSE/SquashFS libraries; it is not a
self-contained universal Linux bundle.

See the [source-and-artifact manifest](acceptance/p5f-w3-packaging/release-manifest.json)
and [actual APT result](acceptance/p5f-w3-packaging/deb-install.json). The manifest
binds both final artifact hashes to **2,008 inputs**. The superseded first round
is [archived separately](acceptance/p5f-w3-packaging/round-1-6e35f70/README.md).
Independent final installer review passed. The actual AppImage also stayed
alive for 15 seconds in an isolated virtual display without input automation;
this verifies graphical-process startup, not interactive desktop workflows.

## Remaining delivery gates

The verified `f6f6cf9` Linux candidates remain the previous integrated package
checkpoint. The subsequent palette, menu, preferences and workspace reflow fixes
are accepted in source; future installer builds must include them and repeat
package verification.

| Item                    | Remaining work                                                                                                                                                                                                                   |
| ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Row 9: native parity    | Build and execute this code on real macOS and Windows; verify platform input behavior without prohibited global desktop automation. Linux headless browser and simulated modifier tests are not native parity evidence.          |
| Row 62: installation    | Linux candidate installation is accepted within the limits above; actual macOS DMG and Windows MSI installation/launch/removal remain. Native CI definitions and helpers are reviewed, but have not been run on those platforms. |
| CI access               | The authenticated GitHub account could not resolve either the configured `fontcarver/fontcarver` or the user-supplied `fontvarver/fontcarver` spelling. No replacement repository was created and no workflow was dispatched.    |
| Signing and publication | Deferred by the user. Developer ID/notarization and Windows code-signing credentials are not configured. No release, upload, website download link or signing claim is performed in this local batch.                            |

## Repair and verification work completed in this batch

- The earlier metrics-key move, E2E session isolation and wording review were
  committed locally as `c1bd334`, `86770ab` and `a4be04a`.
- [Real ttfautohint](p5f-w3-hinter-acceptance.md): genuine 1.8.4 execution,
  composite-helper correction and malformed-output rejection; independent
  fontTools, FreeType and HarfBuzz checks. The tool remains an optional external
  program, not an installed or bundled dependency.
- [Spacing](p5f-w3-spacing-acceptance.md): Alt brackets restore ×100 and exact
  one-unit steps while preserving glyph navigation, keymap conflicts and locks.
- [Bitmap workflows](p5f-w3-bitmap-fixes-acceptance.md): native PNG saving and
  actual vertical bitmap metrics/pixels, with explicit unshaped-proof limits.
- [CJK editing](p5f-w3-cjk-editing-acceptance.md): fresh metric reads and guarded
  component submission prevent delayed edits overwriting current data.
- [Fix All](p5f-w3-fix-all-acceptance.md): multiple safe repairs in one gesture,
  token-scoped cancellation and preservation of previous undo/redo history.
  [Caller regressions](p5f-w3-cancel-callers-acceptance.md) also correct width/RSB
  drag previews and Escape behavior.
- [Open dialog](p5f-w3-open-filter-acceptance.md): nine already-supported format
  extensions are selectable. Type 42 and bare CFF were already implemented;
  the former TODO claiming they were unbuilt was stale.
- [Independent translation](p5f-w3-translation-acceptance.md): GPT-6.1 Sol
  reviewed all 12 new/changed values at the 3,943-key checkpoint. The later UX
  and [welcome delta](p5ux-start-discovery-translation-acceptance.md) were also
  separately reviewed; English and Chinese now have **3,945 matching keys**.
- All 112 rows were independently traced against current code and evidence:
  [rows 1–56](p5f-w3-audit-a.md), [rows 57–112](p5f-w3-audit-b.md).
  New ordinary defects found by that review were repaired and separately reviewed.

## Future experience and feature backlog

Requested for the backlog on 2026-10-02. These are optional future work, not
requirements added to the current local verification and installer batch.
Choose and scope each item separately before implementation; the current
bounded acceptances below remain unchanged.
The user subsequently authorized and completed the welcome-screen discovery,
illustrated guide, bounded 200% welcome/palette check, menu repair and
preferences/shortcut-settings and workspace/Font Info/Export checks above. The broader
first-use research and other items remain future work. Stop after the current
task as requested; no further item has been selected for implementation.

### Experience polish

- [ ] Improve first-use guidance and discovery of tools and shortcuts; observe
      whether new users can open a font, edit a glyph, undo, save and export without
      assistance, then address the specific obstacles found.
- [ ] Validate trackpad scrolling, zoom and gesture interactions on real hardware.
- [ ] Check high-DPI displays, display scaling and larger UI text for readable
      labels, reachable controls, visible focus and unclipped dialogs.
- [ ] Exercise long editing sessions with large fonts; measure responsiveness,
      memory growth, save/recovery behavior and workspace comfort.
- [ ] Collect trial feedback from practicing type designers using real projects;
      record recurring problems and prioritize improvements by observed impact.

### Longer-term feature expansion

- [ ] Extend variable color-font authoring and export beyond the accepted paint
      and variation boundaries, with explicit round-trip/rendering acceptance cases.
- [ ] Add color-aware proofing across supported color formats and instances.
- [ ] Expand semantic TTX editing beyond the currently supported tables, with
      loss/preservation behavior documented for each additional table.
- [ ] Extend geometry workflows beyond the documented sampled, threshold and
      affine approximations; define correctness and performance cases first.
- [ ] Deepen font-family workflows, including broader master/instance management
      beyond the current axis-only virtual-master boundary.
- [ ] Broaden plugin workflows and the available catalogue, with compatibility,
      discovery and lifecycle behavior specified for each increment.

## Accepted boundaries and test conditions

The existing bounded feature acceptances remain in force. Notable later product
work includes variable color paints/color proofs, general semantic TTX,
ascending-only inventory sorting, axis-only virtual masters and broader plugin
catalogues. Python scripting needs a local interpreter, and an already dispatched
synchronous kernel call cannot be interrupted. Geometry retains its documented
sampled/threshold/affine approximations; CFF2 has no stem/mask/flex operators and
TrueType authored glyph programs require a static export. CID identity mappings,
manual IDS input, color-strike precedence and whole-font bitmap color export keep
their documented boundaries. None is silently promoted by passing regression tests.

E2E tests each own a kernel process/profile and font sidecars. They use one worker
per API/UI port pair, a prebuilt CLI (no implicit Cargo compilation), a 1440×1000
viewport and disabled browser sandbox in this environment. The external Source
Han Sans font remains outside the repository. Current full-suite outcomes,
including any timing outlier, are recorded in the integrated and latest
[E2E acceptance](p5ux-latest-e2e-acceptance.md); the earlier 100-test milestone has
its own [historical report](p5f-w3-e2e-acceptance.md).
