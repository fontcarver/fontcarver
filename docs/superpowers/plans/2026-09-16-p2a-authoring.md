# P2a horizontal kerning authoring implementation

Date: 2026-09-16. Authorized by the user's request to complete all of P2a.
Binding behavior: [P2a design](../specs/2026-09-07-p2-kerning-design.md).
Existing stored resolver and ownership check are the starting point.

## 1. Document transactions

Files: `fc-doc/src/kerning/`, `command/kerning.rs`, `command.rs`,
`command/apply.rs`, `document.rs`, `undo.rs`, and focused document tests.

Public commands (through the existing `doc.apply`/edit-session pipeline):

- `SetHorizontalKerning { master, left, right, value: Option<f64> }`.
  `None` removes a record; `Some(0)` is an explicit exception.
- `SetKerningGroup { side, name, members }` creates/replaces one group's members.
- `CreateKerningGroup { side, name, members }` atomically rejects an existing
  name; the inspector uses this for its create-and-assign transaction.
- `AssignKerningGroup { side, glyph, group: Option<String> }` moves membership
  atomically or unassigns it.
- `RenameKerningGroup { side, from, to }` updates every horizontal pair reference.
- `RemoveKerningGroup { side, name, remove_pairs }` rejects in-use removal unless
  pair removal was explicitly requested.

`KerningGroupSide` is `side1`/`side2`. Names retain canonical UFO prefixes.
Every command checks source ownership and affected references before mutation.
No-op kerning commands preserve generation, history, events and dirty state.
Dedicated kerning snapshots and a `Change.layout` flag avoid outline invalidation.
Glyph rename/removal must preserve/restore exact pair and group references.
Group commands retain vertical pair values unchanged. Tests cover failures,
mixed batches, master isolation, explicit zero, coalescing and provenance.

## 2. Typed API queries

Files: `fc-api/src/types/kerning.rs`, `handlers/kerning.rs`, dispatcher and TS generator.

- `kerning.status`: eligibility and structured blocking reason.
- `kerning.glyph`: first/second side membership and validation diagnostics.
- `kerning.resolve`: concrete glyph names, master, memberships, stored value
  and winning record, plus generation and eligibility.
- `kerning.pairs`: bounded page of stored records for a master.
- `kerning.groups`: bounded page of group summaries/members for a selected side.
  Queries use offsets/limits and an optional expected generation; stale page
  generations are rejected. Member lists also need bounds so a large group
  cannot force a complete-font transfer.
- `kerning.members`: a generation-checked bounded member page.

Mutations remain ordinary typed document commands. Unsupported authoring
cannot bypass the document check. Expose invalid imported records as diagnostics,
retain them in documents, and do not expand class pairs into Cartesian products.

## 3. Compiler and persistence

Files: `fc-compile/src/source.rs` and focused compiler/API/format tests.

Prove actual shaping at 600-unit advances: -80 gives 1120 with kern enabled,
1200 disabled; removal restores 1200. Test group pair -100, glyph/group -60,
group/glyph -40, glyph/glyph zero and each removal. Correct compiler exception
conversion only where tests show a mismatch. Invalid/unresolved source records
are retained on read/save but omitted from compile with diagnostics.
FCF and UFO round trips, undo/redo, rename, delete and master selection must
preserve semantics. Existing imported layout/hinting preservation stays green.

## 4. UI and edit coordination

Files: glyph inspector kerning group panel; text view pair panel, gap geometry,
and a plain TypeScript kerning edit runner; existing edit-barrier integration.

Reuse NumberField, PanelSection, transport and CSS tokens. Provide explicit
glyph-name pair selection plus gap selection for unambiguous horizontal LTR
runs. Show rule scope, inherited/stored values and group impact before edits.
Empty/unassigned groups disable unavailable scopes; zero and remove differ.
Disable ambiguous ligature/mark/RTL gaps with an explanation, retaining preview.

Mod+Alt+Left/Right changes the selected pair by 1, Shift by 10, macOS extra
Control by 100. Keys coalesce for 500 ms. Inputs and modals keep normal behavior.
Drain pending edits before document/pair/master/mode/save/export/close changes;
reject late responses against request identity. Kernel shaping is authoritative;
do not add client-side spacing. Display pending/failed compilation accurately.

## 5. Localization and acceptance

All new visible text uses i18next; Chinese translation follows the separate-pass
requirement in `CONVENTIONS.md`, followed by terminology/key-parity QA.
Use focused unit tests for delayed errors, stale responses and edit handovers.
Use real API/compiler tests plus isolated headless Playwright workflows for
pair/group editing, zero exceptions, repeat keys, undo, save/reopen and export.
Use the agent-browser skill for visual checks of existing design integration,
light/dark themes and keyboard reachability. Native desktop automation remains
paused, as do performance investigation and FontForge comparison.

## Verification and handoff

Run relevant tests per slice. At the milestone run workspace formatting/Clippy,
Rust suites appropriate to all changed crates, generated API/TypeScript checks,
ESLint, app tests and headless acceptance. Review diff and record measured
results/remaining boundaries in `docs/p2a-acceptance.md`. Keep the existing dirty
workspace intact; do not merge or publish as part of this request.

## Progress

- [x] Document pair/group transactions and regression tests.
- [x] Typed API, pagination, diagnostics and tests.
- [x] Compiler exception and persistence integration.
- [x] Glyph group and text pair UI, edit coordination and tests.
- [x] Chinese translation and localization QA.
- [x] Headless/visual acceptance and final verification.
