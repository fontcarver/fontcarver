# End-to-end session isolation

Date: 2026-10-02. Branch: `p5-color-authoring`. Validation recorded before commit.

Each Playwright test now owns a kernel process and preference directory.
Default-font tests import a private DejaVu copy; source workflows start empty.
Autosave is disabled before startup. Teardown closes browser contexts, terminates
the owned kernel (including unfinished imports), and removes its profile/font.
All four configurations use this fixture; existing servers cannot be reused.
Generated hint fonts and screenshots use individual test output paths.

The regression leaves dirty documents, changed preferences and an unfinished
import after closing its page. Before isolation, its successor saw two documents
and 6,254 glyphs: **1 passed, 1 failed**. With isolation it sees one clean
6,253-glyph document and default preferences, including across repeated runs.

Additional harness fixes wait for save/conversion/undo responses, focus the
canvas for history shortcuts, let click resolve virtualised cells, and bound
performance sampling by the existing test deadline. The 8 ms paint limit is
unchanged. Recovery-dialog dismissal workarounds were removed.

Verification (commands/output: [validation log](p5f-e2e-isolation-validation.txt)):

- Fresh `cargo build -p fc-devserver` and `cargo build -p fc-cli`: passed.
- `pnpm check`, `pnpm lint`, `pnpm exec prettier --check .`: passed.
- `pnpm test`: **339 files, 2,775 tests passed**.
- Full zero-retry suite: **91 passed, 5 failed, 1 skipped** (external CJK corpus).
  Save/performance timing fixes then passed **2/2**.
- Five-file repetition: **42/44**; subsequent undo/conversion fixes passed
  **6/6**, three repetitions with traces. Isolation passed both repetitions.
- Dedicated-configuration and removed-workaround smoke checks: **13/13 passed**.

Three download failures remain environment-dependent: a standalone blank-page
Blob download also returns `canceled`, without FontCarver. The complete suite
is therefore **not green**. An earlier cleanup experiment was interrupted,
not passed. Translation remains for the user's future GPT-6.1 Sol sub-agent.

Resources: build cache **14.16 → 14.91 GiB**, within budget. Owned servers
stopped; task scratch/profile removed. Final artifact check: **PASS**.
