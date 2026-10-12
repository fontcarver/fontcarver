# p5f-e — OpenType features and layout acceptance

Date: 2026-09-30. Area `e-features`, rows 46–51, 72, 99 and 100 of the
[feature audit](p5-feature-audit.md). Protocol:
[`p5f-protocol`](superpowers/plans/tasks/p5f-protocol.md). Task file and gap
analysis: [`p5f-e-features`](superpowers/plans/tasks/p5f-e-features.md).
No commits were made and no status table was edited.

## Scope

This task closes the gap between "implemented" and "accepted" for the feature
editor: a typed FEA lookup IR with a proven round trip, per-entry automatic and
manual authoring, the complete enumerated set of automatic recipes, verified
model-generated `kern`/`mark`/`mkmk`/`curs` precedence, a measured incremental
layout compile, and browser evidence for the authoring workflows.

## Delivered

### Row 46 — FEA parse/serialise round trip into a typed lookup IR

New owned module `crates/fc-layout/src/fea_ir/` (`mod.rs` types, `parse.rs`,
`write.rs`). `parse_fea(source, glyph_order)` walks the public `fea-rs` concrete
syntax tree (`Kind`, `Node`, `Token`; the `typed::*` accessors are `pub(crate)`)
and produces `FeaIr`: ordered statements covering `languagesystem`, `@class`
definitions with ranges/CIDs/class references, `markClass`, `lookupflag` in both
the numeric and named forms, `script`/`language` with their default-language
policy, `subtable`, named `lookup` blocks with `useExtension`, `lookup` calls,
`aalt` feature references, comments, and typed rules for **GSUB 1/2/3/4/5/6/8 and
`ignore sub`** plus **GPOS 1/2/3/4/5/6/7/8 and `ignore pos`** — the nine required
types plus cursive, mark-to-ligature and reverse chaining. Each rule carries its
`LookupType` (`GSUB 4`, `GPOS 6`, …). A node whose shape does not match is kept
as `Statement::Verbatim` holding its exact source text, so nothing is dropped.
`write_fea` renders canonical syntax, escaping every glyph name that can be
escaped, and the pair is a semantic round trip.

`fc-layout` now depends directly on `fea-rs` 1.0 — already a workspace
dependency of `fc-compile`, so no new third-party crate or licence question.

### Row 47 — per-entry automatic versus manual

`FeatureGenerationSettings` gains `manual_entries` (feature tags and class names
held manual inside an enabled recipe) and `automatic_classes` (opt-in generated
glyph classes), plus `family_enabled`, `set_family_enabled`, `entry_is_manual`
and `generator_for_entry`. `freeze_feature_source` takes an optional `entry`:
with it, only that tag or class becomes manual — keeping its generated text —
and the recipe stays enabled for its other entries. `features.freeze` carries the
new `entry` field, and `FeatureState.automatic_entries` tells the editor every
name a recipe can own, so the frontend mirrors no registry.

In the editor every prefix, class and feature entry now has an **Automatic**
switch. Switching off freezes that entry alone. Switching on asks for
confirmation first, then enables the recipe, clears the manual hold, holds the
family's other user-owned entries manual, and removes the user's text so the
recipe can materialise the entry. Prefixes, unknown tags and the two
mapping-driven recipes (`locl`, `dlig`) show the switch disabled with the reason
both on hover and as plain text tied to the control with `aria-describedby`,
because a disabled checkbox takes no focus and shows no tooltip.

A draft edit made after a check — adding or renaming an entry — now stays
visible and selectable until the next check refreshes the generated preview;
previously the entry was created but the stale preview hid it.

### Row 48 — generation from glyph-name suffixes

Recipe rule building moved into the owned model module
`crates/fc-model/src/features/generators.rs`; `fc-compile`'s existing
`generate_feature_source` dispatches to it for the new families through a narrow
additive patch, and the seven P2b families keep their existing implementation.
The complete enumerated set is now generated:

`smcp`, `c2sc`, `ss01`–`ss20`, `case`, `vert`/`vrt2`, `liga`, `dlig`, `locl`,
`lnum`, `onum`, `tnum`, `pnum`, `sups`, `sinf`, `numr`, `dnom`, `frac`, `ordn`,
`ccmp`, `aalt`, plus generated glyph classes (`@Uppercase`, `@Lowercase`,
`@Letters`, `@Figures`, `@Marks`, `@Punctuation`, `@Symbols`).

`c2sc` uses `.c2sc` glyphs and lowercase-based small capitals (`A → a.sc`, found
through the base glyph's code point), so it never duplicates `smcp`'s `A.sc`
rules. `frac` emits generated figure classes and a reverse-chaining numerator
lookup plus a forward-chaining denominator lookup, so any digit count converts on
both sides, and a plain `slash` between figures becomes the fraction slash.
`ordn` combines the classic `ordfeminine`/`ordmasculine` rules after a figure
with `.ordn` glyphs. `ccmp` reads real component structure — a base plus mark
components under plain translation becomes the precomposed glyph — and adds the
dotless substitution before a mark that sits **above** the base, decided by the
mark's own `_top`-prefixed anchor when the document has one and otherwise by its
code point, so a cedilla or a dot below leaves the dot of `i` in place. `aalt`
references the alternate-producing features actually present, in a fixed order.

Generated entries are named after what they are — the feature tag or the glyph
class name. Only the helper classes and lookups a recipe writes inside generated
code use the reserved `fcGen` prefix (`@fcGenFracNumerator`,
`lookup fcGenFracNumerators`).

A collision with a feature the user owns is reported for **that tag alone**; the
rest of the family still materialises, so enabling a recipe for one entry cannot
silently lose its siblings. Switching one entry to automatic also holds every
other entry of that family the user still owns manual in the same change, so a
hand-written `onum` survives when `lnum` is handed to the recipe.

### Row 49 — implicit `kern`/`mark`/`mkmk`/`curs`

`crates/fc-layout/tests/model_layout_writers.rs` compiles and shapes real fonts
to prove all four tags, including class-pair kerning with a glyph-pair exception,
a two-mark stack (`mark` plus `mkmk`) and `entry`/`exit` cursive attachment, and
proves the precedence ladder: an authored feature owns its tag; `# Automatic
Code` merges the model's rules; mixing the marker with authored rules of the same
kind is rejected before activation; and turning a writer off hands the tag back
without touching the data. The ladder is documented for users.

### Row 50 — diagnostics, autocomplete, class preview

Verified in the browser: located diagnostics with jump-to-source, completion, and
the Alt-click / Alt+Enter read-only class preview with rendered outlines
(screenshot below, and a Playwright scenario).

### Row 51 — `vert`/`vrt2`

`.vrt2`-suffixed alternates are now recognised and generated into `vrt2` only,
while `.vert` still feeds both tags. A vertical shaping test and a browser
scenario prove `vert` applies by default in a vertical run, not in a horizontal
one, and that it can be switched off.

### Row 72 — incremental GSUB/GPOS-only shaper

`Shaper::used_incremental_layout` makes the live cache's build path observable.
`crates/fc-layout/tests/incremental_layout_budget.rs` measures a 400-glyph font:
a layout-only refresh must be under 750 ms **and** less than half a full compile,
must keep `glyf`, `loca`, `cmap`, `hmtx`, `maxp`, `post` byte-identical and
`head` identical apart from the checksum adjustment, and must refresh GSUB/GPOS.
It also proves the live `fc-layout` shaper cache takes the incremental path after
a feature-only edit and falls back to a full build after an outline edit.

Both timings are best of three, and the assertion is the loose
`incremental * 3 < baseline * 2` so an ordinary CI machine cannot flake.
Measured here under a loaded machine: **full compile 140.2 ms, layout refresh
25.4 ms** for 400 glyphs (5.5×); 20 consecutive refreshes of a 200-glyph font in
361 ms.

### Row 99 — visual lookup editor

Backend acceptance (`fc-api --test visual_lookup_authoring`, 12 tests covering
all nine kinds through validation, shaping, undo/redo and `.fcf` persistence)
was recorded as _interrupted_ by the earlier batch; it is now observed passing.
A Playwright scenario authors one rule per supported type in the visual editor
and checks the rendered FEA for each, then applies and shapes the result.
`fixtures/visual-lookup-corpus.json` is the single source of those strings:
`app/src/views/features/visualLookup.test.ts` checks that the editor serialises
each rule to that exact text and parses it back into the same rule, and
`crates/fc-layout/tests/fea_ir.rs` checks that the same text parses into the
typed IR with no verbatim statement and round trips. Neither side can drift.

### Row 100 — tokens and `#ifdef VARIABLE`

The model's dialect already compiled; the editor now shows it. `featureLanguage`
marks whole-line `#ifdef`/`#ifndef`/`#else`/`#endif` as directives rather than
comments, highlights `$name`, `${expression}` and `$[predicate]` tokens, marks an
unterminated token invalid, and completes both the directives and the built-in
metric tokens. A completion bug found during browser verification — a complete
directive kept its popup open so Enter accepted it instead of inserting a
newline — was fixed and covered by a test.

## Commands and results

All Rust commands ran under `flock .scratch/p5f/cargo.lock` with
`TMPDIR` inside `.scratch/p5f-e/tmp`.

| Command                                                                                                                                                                      | Result                                |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| `cargo test -p fc-layout`                                                                                                                                                    | 128 passed, 0 failed (11 binaries)    |
| `cargo test -p fc-model`                                                                                                                                                     | 109 passed, 0 failed (18 binaries)*   |
| `cargo test -p fc-compile --test feature_generation --test feature_writers --test feature_source --test feature_validation --test feature_tokens --test shaping_incremental` | 34 passed, 0 failed                   |
| `cargo test -p fc-api --lib`                                                                                                                                                 | 234 passed, 0 failed                  |
| `cargo test -p fc-api --test features --test visual_lookup_authoring --test layout_inspection --test layout_conversion`                                                      | 36 passed, 0 failed                   |
| `cargo test -p fc-doc` / `cargo test -p fc-formats`                                                                                                                          | 325 / 189 passed, 0 failed            |
| `cargo clippy -p fc-model -p fc-compile -p fc-layout --all-targets -- -D warnings` (run per crate)                                                                           | clean                                 |
| `rustfmt --edition 2021 <owned files>`                                                                                                                                       | clean                                 |
| `pnpm --dir app exec vitest run --maxWorkers=1 src/views/features`                                                                                                           | 21 files, 164 passed                  |
| `pnpm --dir app check`                                                                                                                                                       | clean for this task's directories\*\* |
| `pnpm --dir app exec eslint src/views/features e2e/p2b*.ts`                                                                                                                  | clean                                 |
| `pnpm --dir app exec prettier --write src/views/features e2e/p2b* src/i18n/en.json`                                                                                          | clean                                 |
| `FC_API_PORT=8805 FC_UI_PORT=1505 pnpm --dir app exec playwright test --config e2e/p2b.config.ts`                                                                            | **17 passed, 0 failed**               |

New Rust tests added by this task: `fc-layout` `fea_ir` (11),
`feature_generation` (4), `model_layout_writers` (4),
`incremental_layout_budget` (3); `fc-model` generator unit tests (9) and a
settings round-trip test (1); `fc-api` handler tests (2). New frontend tests:
`automaticEntries` (5), `FeatureAutomaticSwitch` (3), `featureLanguage` (5),
`visualLookup` shared-corpus (1).
New Playwright scenarios: `app/e2e/p2b-recipes.spec.ts` (7).

`cargo clippy -p fc-api --all-targets -- -D warnings` reports three
`unnecessary if let` errors in `crates/fc-api/src/handlers/variation.rs`, a file
owned by another task and untouched here; every file this task owns is clean.

\*\* `pnpm --dir app check` reports two errors in `src/shell/HistoryPanel.*`
from another task's in-flight change; no error names this task's directories.

\* The full `fc-model` figure is the last complete run of this task. Three later
retries, spaced minutes apart as the protocol requires, stopped at a compile
error in `crates/fc-model/tests/serde_roundtrip.rs` (`missing field `overrides`in initializer of`Instance``) from another task's in-flight model change. The
suites this task touches were rerun afterwards and pass:
`--lib` 34, `feature_tokens` 4 + 9, `feature_assembly` 6, `invariants` 6,
`flatten`/`designspace` 20.

## Browser evidence

Interactive verification used the isolated ports 8805/1505 with a UFO fixture
built from the project's licensed `Minimal.ufo`. Both servers and the browser
session were stopped afterwards.

- [Generated recipes](screenshots/p5f-e/generated-recipes.png): twelve recipe
  switches, seventeen generated entries in the **Automatic** section, the
  selected `smcp` entry read-only with its provenance and its per-entry
  **Automatic** switch checked.
- [Automatic-entry confirmation](screenshots/p5f-e/automatic-entry-confirm.png):
  `ss01` held manual sits under **Features** while the other recipe entries stay
  automatic; switching it back asks before replacing its source.
- [Visual lookup editor](screenshots/p5f-e/visual-lookup-editor.png): a GPOS 4
  mark-to-base rule authored through typed fields.
- [Class preview](screenshots/p5f-e/class-preview.png): Alt+Enter on `@Caps`
  showing its three members with their outlines.
- [Tokens and conditionals](screenshots/p5f-e/tokens-and-conditionals.png):
  directives distinguished from the comment above them, `${…}` and `$[…]` tokens
  highlighted, and "Source compiles in 1 master."

Kernel checks behind those screenshots: after compiling the token source,
`text.shape` returns an advance of 540 for `A` (600 − `A:width / 10`), and the
per-entry switch round trip leaves `manual_entries` empty with `ss01` generated
again and 17 generated records.

## Independent output checks

The typed IR round trip is checked against **real compiled binary layout**: six
feature sources are compiled by `fontc`, the binary is read back by
`LayoutImport`, decompiled to FEA, and that decompiled text must parse, render
and reparse to an equal IR with every lookup typed (only whole `table` blocks stay
verbatim). The licensed fixture fonts keep their documented P2b conversion
blockers, so a separate test records which of them can be decompiled at all and
asserts the rest have a recorded blocker rather than implying coverage.

Every generated recipe is checked by compiling the candidate and shaping it with
HarfRust, comparing glyph names with the feature on and off, in horizontal and
vertical runs.

## Remaining limits

- **Compound figure suffixes.** `.tosf` and similar names that belong to two
  figure features at once are not auto-assigned; single-purpose suffixes are.
- **`ccmp` composition** is derived from component structure and a dotless table,
  not from Unicode decomposition data, which the model layer does not carry. A
  precomposed glyph drawn as contours, or built with a scaled or mirrored
  component, is not composed. The dotless rule's above-mark test uses explicit
  `_top` anchors when present and otherwise a built-in table of the
  combining-class-230 ranges a Latin base can meet, not the full Unicode table.
- **`locl` and `dlig`** remain explicit mapping lists (the accepted P2b decision);
  they are never guessed from language suffixes.
- **Generated names.** Generated entries take the feature tag or class name they
  represent; only the helper classes and lookups written inside generated code
  carry the reserved `fcGen` prefix. A user source item with the same generated
  identity is refused with `feature-generation-identity-conflict`; a user-owned
  tag or class name is refused with `feature-generation-conflict` for that name
  only; a clash with an `fcGen` helper name is reported by the compiler at
  validation time rather than by a dedicated pre-check.
- **The typed IR is not an editing surface yet.** It is a kernel capability with
  an internal API; the visual editor still parses its bounded subset in
  TypeScript, and a shared corpus test keeps the two in agreement. Feature
  variations, `conditionset`, `anchorDef`, `valueRecordDef`, `anon` and whole
  `table` blocks are preserved verbatim rather than typed. `include(...)` is the
  one exception: it is a hard error with an actionable message, because the
  feature editor and the compiler both read one self-contained source and
  disable external includes, so the IR must not pretend to have read a file it
  never opened.
- **Incremental shaping budget** is measured on this machine at 400 glyphs, best
  of three on each side; it is a relative and absolute regression guard, not a
  release performance claim. The byte-identity assertions are the primary check.
- **Reopening a saved document.** The shell now focuses a path it already has
  open instead of opening it twice, so the invalid-draft scenario reopens the
  saved bytes through a copy of the file. The claim it proves is unchanged: the
  draft comes back from disk.
- **Replacing an entry's text.** Switching a manual entry to automatic discards
  its text after the confirmation, as the dialog says. It is one undo unit, and
  **Discard draft** also restores it, but if the recipe turns out to produce no
  rules for that name the entry simply disappears from the preview until then.
- Native desktop automation and packaging remain out of scope, as for every P5
  area.

## Recommended per-row status

| Row | Recommended                                                                                           |
| --- | ----------------------------------------------------------------------------------------------------- |
| 46  | **Complete** within the stated verbatim-preservation boundary                                         |
| 47  | **Complete**                                                                                          |
| 48  | **Complete** with the compound-suffix and `ccmp` limits recorded                                      |
| 49  | **Complete**                                                                                          |
| 50  | **Complete**                                                                                          |
| 51  | **Complete**                                                                                          |
| 72  | **Complete**                                                                                          |
| 99  | **Complete** for the nine supported types; arbitrary FEA and context graphs stay in the source editor |
| 100 | **Complete** within the documented dialect                                                            |

## Resources

`python3 scripts/check_artifacts.py` was run at task start, between build
batches and before this report. The owned dev server (8805), Vite (1505) and
browser session were stopped, and `.scratch/p5f-e/` was removed. No source file,
font, shared cache or other task's artifact was deleted.
