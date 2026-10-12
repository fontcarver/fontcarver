# P5-85 construction editing surface

## Scope

Expose reusable glyph constructions (row 83), node-owned interpolation (row 86),
and guide-owned node links with magnetic attachment and automatic axis-aligned
straight stem detection (row 85). Kernel and persistence ownership are described
in sibling task plans. This UI owns only new construction panel/helper/test files
and a narrow EditInspector insertion. All new visible strings use the English
`construction` namespace. Localization belongs to the independent translator.

## Interfaces

Read authoritative layer metadata from the active glyph's matching layer. Commit
construction and node-link commands through `commitField` for one undo entry and
normal error/reload behavior. Bind selection using stored PointRef indices at the
command boundary; persistent identities are minted and validated by the kernel.
Reuse ordinary layer guides for snapping and editing. Magnetic attachment keeps
the selected nodes' signed distances; guide movement updates the nodes.

## Automatic stems

Detect pairs of parallel straight horizontal/vertical outline edges, with
positive overlap and bounded width. Test the interior across the overlap and use
nonzero fill winding to exclude counters. Show a preview count before creating
one pair of linked edge guides per stem. Deduplicate shared edges, skip already
linked guides, cap work on very complex outlines, and clearly label the straight
axis-aligned scope. No inferred stem is silently committed on selection changes.

## Acceptance

- Tests cover stems/counters/open and degenerate outlines, pair deduplication,
  selection-to-command construction, invalid selections, and UI dispatch.
- Servant controls expose X and Y and show existing relationships; component
  controls expose source, kind, scale/baseline/spacing and removal.
- Headless agent-browser renders the panel and exercises the control surface;
  no native/global desktop input is used.
- Focused Vitest and relevant TypeScript checks; parent runs final regression.
- Run artifact budget checks and remove only this task's scratch outputs.
