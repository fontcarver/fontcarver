# Font Cutter Editor UI — Design Specification

Owner: Fable (design). Implementation: Opus (P1 Tasks 6–9, 11). Status: v1, 2026-09-03. This document is binding for every UI task; the P1 plan's task briefs point here for look, feel and interaction grammar. Where this document and the competitor research disagree, this document wins; where it is silent, follow Glyphs 3 conventions.

## 1. Principles

1. **The glyph is the hero.** Everything around the canvas is quiet: neutral surfaces, thin borders, one accent. Colour on the canvas means something (node type, selection, warning); chrome never competes with ink.
2. **Grammar over features.** One modifier grammar for the whole app: ⇧ constrain / ×10, Ctrl (⌘ on macOS) ×100, ⌥ "the other behaviour" (ignore handles, from centre, duplicate, all masters), Ctrl+⌥ nudge, ⌃/Ctrl-hold suspends snapping. Never overload ⌥ twice on one tool.
3. **Intent before click.** The cursor and a ghost preview tell you what a click will do before you click: insert-node ghost on a segment, move cursor on a node, break-handle cursor with ⌥, ghost component nodes when you approach a component.
4. **Numbers are first-class.** Every draggable value has an editable field (Info box / inspector) with ↑↓ increments; every field edit is one undo step; tabular numerals everywhere.
5. **Nothing is modal that doesn't have to be.** Panels and the Info box are non-modal; dialogs are reserved for file operations and irreversible batch actions.

## 2. Design system

### 2.1 Colour tokens (`app/src/styles/tokens.css`)

| Token | Light | Dark | Use |
|---|---|---|---|
| `--bg` | `#F3F4F6` | `#15181C` | window ground |
| `--surface` | `#FFFFFF` | `#1E2228` | panels, dialogs, cells |
| `--surface-2` | `#F8F9FA` | `#23282F` | toolbar/tab strips, table headers |
| `--line` | `#D9DDE3` | `#2C323A` | 1 px borders, gutters |
| `--line-strong` | `#B8BEC7` | `#3A424C` | focused borders, active tab underline |
| `--text` | `#1B1F24` | `#E6E8EB` | primary text |
| `--muted` | `#5C6470` | `#9AA3AD` | secondary text (≥ 4.5:1 on surface) |
| `--faint` | `#7A828E` | `#7C8590` | tertiary, only ≥ 13 px |
| `--accent` | `#3B6FB6` | `#7FA6DC` | selection, primary buttons, links |
| `--accent-ink` | `#FFFFFF` | `#0F1418` | text on accent |
| `--accent-soft` | `#E4ECF7` | `#243448` | selected rows, chips |
| `--ok` / `--warn` / `--danger` | `#2E8B57` / `#B7791F` / `#C2413B` | `#5DBB84` / `#E0A23C` / `#E36D66` | status only, never as accent |
| Canvas: `--cv-bg` | `#FFFFFF` | `#1A1D21` | glyph canvas ground |
| `--cv-ink` | `#1B1F24` | `#E6E8EB` | outline stroke / filled preview |
| `--cv-metric` | `#B8BEC7` | `#3F4750` | metrics lines |
| `--cv-metric-label` | `#7A828E` | `#8B94A0` | metrics labels |
| `--cv-guide-local` | `#3B6FB6` | `#7FA6DC` | local guides (accent) |
| `--cv-guide-global` | `#C2413B` | `#E36D66` | global guides (red, Glyphs convention) |
| `--cv-node-corner` | `#3B6FB6` | `#7FA6DC` | corner nodes (blue square) |
| `--cv-node-smooth` | `#2E8B57` | `#5DBB84` | smooth nodes (green circle) |
| `--cv-handle` | `#8E96A2` | `#7C8590` | off-curve points and handle lines |
| `--cv-selection` | `#2B5FA8` | `#9BC0F0` | selected nodes/handles fill, marquee |
| `--cv-component` | `#8E96A2` | `#6E7783` | component outlines (muted) |
| `--cv-background-layer` | `#C9A227` | `#D4B14A` | background layer outline (ochre, 60 % alpha) |
| `--cv-other-master` | `#9E7BB5` | `#B394C9` | other masters overlay (violet, 50 % alpha) |
| `--cv-anchor` | `#C2413B` | `#E36D66` | anchors |
| `--cv-warning` | `#C2413B` | `#E36D66` | duplicate nodes, audit markers |
| `--cv-zone` | `#EADFB8` | `#3A3520` | alignment zones fill (30 % alpha) |
| `--cv-hud-bg` | `rgba(27,31,36,.92)` | `rgba(230,232,235,.94)` | cursor HUD (inverted) |

Rules: one accent; semantic colours never used decoratively; every canvas colour has a light and dark value and is read through `useCellTheme`/`glyphTheme`; no hard-coded colours outside `tokens.css` and `glyphTheme.ts`.

### 2.2 Typography

- UI: `system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", "Noto Sans", "PingFang SC", "Microsoft YaHei", sans-serif`. Scale: 11 (captions, only muted-or-darker), 12 (dense tables, labels), 13 (default), 14 (dialog body), 16 (dialog titles), 20 (empty-state title). Weights 400/500/600 only.
- Numbers: `font-variant-numeric: tabular-nums` on every numeric field, table and HUD; monospace (`"SF Mono", "JetBrains Mono", "Cascadia Mono", ui-monospace`) only for glyph names, unicode codes and code (FEA in P2).
- Chinese UI: same scale; never below 12 px for CJK text.

### 2.3 Spacing, shape, elevation

- Spacing scale 4 / 8 / 12 / 16 / 24 / 32. Panel padding 12; row height 24 (dense lists) / 28 (fields); toolbar height 32; tab height 30; status bar 24.
- Radii: 4 (buttons, fields), 6 (dialogs, popovers), 3 (chips). No radius on panels and canvases.
- Elevation: borders, not shadows, for docked chrome. Popovers/dialogs: `0 8px 24px rgba(0,0,0,.18)` light / `.45` dark plus a 1 px `--line` border.
- Focus: 2 px `--accent` ring with 1 px offset, visible for keyboard focus only (`:focus-visible`).

### 2.4 Icons and cursors

- Lucide icons at 16 px (1.5 px stroke), colour `currentColor`; tool icons at 18 px in the tool strip. No emoji.
- Cursors: default arrow; `move` on node hover; `crosshair` for pen/knife/shapes; `zoom-in/out`; `grab/grabbing` for hand; custom SVG cursors (24 px, 1 px dark outline + white fill) for: pen (with small "+" when over a segment, "×" when closing a path, "○" when over the start node), knife, measure, rotate, scale, insert-node ("+" over a segment for the select tool), break-handle (⌥ over a node).

### 2.5 Motion

- Durations: 80 ms (hover), 120 ms (panel toggles), 180 ms (dialog in), easing `cubic-bezier(.2,.8,.2,1)`. Canvas interactions never animate (positions follow the pointer exactly). `prefers-reduced-motion`: all transitions 0.

## 3. Application frame (recap of P0, with P1 changes)

```
┌ menu bar (30) ─────────────────────────────────────────────────────┐
├ tab bar (30): [Font] [A] [Hamburgefonstiv ×] [Font Info ×]   ⟳ ▣ ▤ ┤
├ sidebar (200–400) ┬ content ───────────────────────┬ inspector (240–420) ┤
│ Font tab: filters │ Font view / Text·Edit tab      │ context panels      │
│ Edit tab: layers  │  ┌ tool strip (36) ┐ canvas    │ (accordion)         │
│  + glyph list     │                                │                     │
├ status bar (24): doc · glyphs · selection · zoom · coords · kernel ─────┤
```

P1 changes:
- The **Edit tab** hosts both single-glyph and text editing (Glyphs model). A tab shows the glyph name for one glyph or the first 24 characters of the text; dirty state is a dot before the title; middle-click closes.
- **Sidebar in an Edit tab** shows the Layers list (masters, brace/backup/background layers of the current glyph) on top and a compact glyph list (the current filter of the Font view) below, so the user can jump glyphs without leaving the editor.
- **Tool strip**: a 36 px vertical strip on the canvas's left edge with grouped tools (see §5); tooltips show name + key; the active tool is filled with `--accent-soft` and a left `--accent` bar; a "borrowed" tool (held key) shows a dotted outline.
- **Status bar** in an Edit tab: `glyph name · U+XXXX · width · LSB/RSB · zoom % · pointer x,y · snapping on/off`.

## 4. The editing canvas

### 4.1 Drawing order (back to front)

1. Canvas ground `--cv-bg`; optional grid (pref: off, 1–100 units; 0.5 px lines `--line` at ≥ 1 px spacing on screen).
2. Alignment zones (`--cv-zone` fill) and metrics lines: baseline, x-height, cap height, ascender, descender, plus vertical origin/advance for CJK; 1 px `--cv-metric`; labels right-aligned at the left canvas edge in 11 px `--cv-metric-label`, merged when coincident ("ascender 880 · vert. origin 880").
3. Background image (if any) at 50 % opacity.
4. Other masters overlay (when enabled): 1 px `--cv-other-master`, no nodes.
5. Background layer: 1 px `--cv-background-layer`, nodes as 3 px dots.
6. Components: filled `--cv-component` at 18 % alpha with 1 px outline; component origin marker (small ⌖) and name label (12 px, collision-avoided); locked components hatch on hover.
7. Outline: 1 px `--cv-ink` stroke (screen-space, crisp); filled preview when Space is held or fill toggled: `--cv-ink` at 100 % with nodes hidden.
8. Handles: 1 px `--cv-handle` lines from on-curve to off-curve; off-curve points 3.5 px circles `--cv-handle` fill.
9. Nodes: corner = 5 px square `--cv-node-corner`; smooth = 5 px circle `--cv-node-smooth`; G2 "super-smooth" (P2) = 6-point star; open-path ends = 6 px perpendicular ticks; start node = 7 px triangle pointing along the path; node on a vertical metric = 7 px diamond `--cv-zone` border; duplicate coincident nodes = 6 px red ring.
10. Selection: selected nodes/handles filled `--cv-selection` with 1 px white inner ring; selected segments 2 px `--cv-selection`; marquee = 1 px dashed `--accent` with 8 % fill.
11. Anchors: 6 px `--cv-anchor` diamond with name label (11 px) offset up-right; mark cloud (P2).
12. Guides: local 1 px `--cv-guide-local`, global `--cv-guide-global`, dashed when locked; name at the left end; angle handle knob (6 px) when selected; measurement guides show intersection distances.
13. Rulers (top/left, 20 px, `--surface-2`, ticks every 10/50/100 units adapted to zoom) and pointer position markers.
14. Ghosts: insert-node ghost (4 px hollow circle) on segment hover; snap target halo (10 px `--accent` at 25 %); alignment lines (1 px `--accent` at 60 %) when a dragged point aligns with another node's x/y; component ghost nodes (3 px `--cv-handle` at 60 %) when the pointer is within 24 px of a component.
15. HUD (see §6.2) and the Info box.

### 4.2 Sizes, hit-testing, snapping

- Node/handle glyph sizes are constant in screen pixels (DPR-aware); hit radius: nodes 5 px, handles 5 px, segments 4 px, anchors 6 px, guides 4 px; precedence on overlap: selected node > node > handle > anchor > guide > segment > component.
- Snap distance 6 px; snapping order: node > handle > anchor > guide > metric > grid > alignment line; holding Ctrl (⌃ on macOS) suspends snapping; the snapped target shows the halo and the HUD reads the snapped coordinate.
- Zoom: 1 %–6400 %, wheel with Ctrl (pinch on trackpad) zooms at the cursor by 1.15 steps; plain wheel scrolls; `Mod+0` fit glyph, `Mod+1` 100 % (1 unit = 1 px), `Mod+=`/`−`; the zoom field in the toolbar accepts a number.
- Pan: Space+drag (when no drag is in progress), middle button, or Hand tool; two-finger scroll pans.

### 4.3 Node grammar (Glyphs-compatible)

- Click node = select; ⇧+click extends; Ctrl+click toggles; drag on empty = marquee (⌥ ignores handles, ⇧ adds to selection).
- Drag node = move node with its handles; ⌥+drag = node only; ⇧ constrains 0/45/90°; Ctrl+⌥+drag = nudge (adjacent unselected handles scale proportionally).
- Drag handle = move handle; ⌥ keeps its angle; Ctrl+⌥ on a smooth node mirrors length+angle to the opposite handle; ⇧ constrains angle.
- Drag segment = reshape (both handles move); ⌥ keeps handle angles.
- Double-click node or Enter = toggle smooth/corner (levels handles); ⌥ keeps the outline.
- ⌥+click line segment = convert to curve (handles at 1/3, 2/3); delete a handle = back to line.
- Delete = remove nodes keeping the path closed (segment reconstructed); ⇧+Delete = remove node and both segments (opens path).
- Arrows nudge 1; ⇧ 10; Ctrl 100; ⌥ nudges without handles; Tab/⇧Tab cycle nodes.
- Click a segment with the Select tool + ⇧ = insert node at nearest extremum/inflection; plain drag on a segment reshapes.
- Escape: deselect; second Escape exits to text mode in a Text tab.

## 5. Tools (tool strip, top → bottom)

| Key | Tool | Icon | Notes |
|---|---|---|---|
| V | Select | `mouse-pointer-2` | default; ⇧V lasso (P2) |
| P | Pen | `pen-tool` | click node / drag curve; ⌥ outgoing handle only; Space repositions; click first node closes; click existing node opens the path there; ⇧+click segment inserts at extremum |
| B | Pencil | `pencil` | freehand → simplified cubic; ⇧ straight lines |
| E | Knife | `scissors` | drag a line; inserts nodes at intersections and splits; ⇧E erase (P2) |
| F / ⇧F | Rectangle / Ellipse | `square` / `circle` | click = numeric dialog; ⇧ square/circle; ⌥ from centre |
| L | Measure | `ruler` | drag shows distance/angle and outline intersections with per-span distances; ⇧ constrains |
| R / S | Rotate / Scale | `rotate-cw` / `scaling` | click sets origin then drag; ⇧ constrains (15° / uniform); numeric fields in the Transform panel |
| Z | Zoom | `zoom-in` | click zoom in, ⌥ out, drag marquee zoom |
| H / Space | Hand | `hand` | pan |
| T | Text | `type` | text mode in the Text tab |

Tap/hold: a tap switches; holding the key ≥ 200 ms borrows the tool until release (FontLab). The strip shows the borrowed state with a dotted outline. Shape tools and Rotate/Scale show their origin/size in the HUD while dragging.

## 6. Info box and HUD

### 6.1 Info box (docked, bottom-left of the canvas, 260 px wide, collapsible with `Mod+Shift+I`)

Context-sensitive rows, all editable, Tab/⇧Tab walk fields, ↑↓ ±1 (⇧ ±10), Enter commits (one undo step), Esc reverts:
- Nothing selected: glyph name (editable), Unicode (editable), width, LSB, RSB, kerning groups (P2).
- One node: X, Y, type (Corner/Smooth toggle), on-curve/off-curve badge.
- One handle: ΔX, ΔY, length, angle.
- Multi-selection: X, Y, W, H (bounding box), 9-point origin picker, proportional lock, count "12 of 48 points".
- Component: name (click opens base glyph), X/Y offset, scale %, rotation, "use my metrics", "auto-align".
- Guide: position, angle (lock), name, LSB/RSB anchoring, measurement toggle.
- Anchor: name (combo with suggestions), X, Y.

### 6.2 Cursor HUD

A small inverted pill (`--cv-hud-bg`, 11 px tabular) following the pointer at +16 px/+16 px during drags only: `Δx 12  Δy −4` for moves, `∠ 30.0°  ℓ 142` for handles, `w 312 × h 84` for marquee/shapes, `⟂ 96` (stem width) when two nodes are selected and dragged apart, snap target name when snapping ("baseline", "guide: x-height"). Never shown when idle.

## 7. Inspector panels (Edit tab)

Accordion sections in this order; each remembers open/closed state; each header shows a count badge where meaningful:
1. **Glyph** — name, unicodes (chips + add), category/sub-category/script (read-only, derived), export toggle, colour label swatch row (12 Glyphs colours), tags, note.
2. **Metrics** — width, LSB, RSB fields; metrics keys row (`=H`, `=|n`, `=n+10`) with live resolved value and an "out of sync" badge + "Apply" button; vertical width/origin when present.
3. **Layers** — list of layers of this glyph: master layers (bold), brace/bracket, backgrounds, backups (timestamped); actions: add background, copy to background, swap with background, add backup, delete; eye icons toggle visibility of background/other masters overlays.
4. **Components** — list (name, offset, scale), reorder by drag, add via search popover (glyph names with thumbnails), decompose, "open base" arrow; auto-align toggle; use-my-metrics radio.
5. **Anchors** — list (name, x, y), add with suggested names (`top`, `bottom`, `ogonek`, `_top`…), delete; clicking a row selects the anchor on canvas.
6. **Guides** — local/global lists (position, angle, name, lock); add from the ruler by ⌥-drag; convert to measurement guide.
7. **Transform** — move (dx, dy), scale (x %, y %, lock), rotate (°), skew (°), flip H/V, origin picker (9-point + custom), "Apply" (one undo step); align/distribute row (left/center/right/top/middle/bottom, distribute H/V) for multi-selection.

Field behaviour: text fields select all on focus; numbers accept expressions (`+10`, `*2`) and units; invalid input shakes the field (120 ms) and shows a tooltip; changes commit on Enter/blur.

## 8. Text tab

- Top **text bar** (32 px): a single-line text input showing the tab's text with a sample-text menu (Latin pangrams, Chinese sample sentences, "All encoded glyphs", "Kerning pairs" P2), size field (px/em), master switcher, feature toggles (`kern`, `liga`, `calt` when present; P2 adds all), direction LTR/RTL/vertical (vertical in P4).
- **Glyph runs** are laid out with shaped advances; the active glyph (clicked) shows full editing chrome, others show outline only at 100 % `--cv-ink` with metrics boxes hidden unless "show metrics for all" is on; `Mod+←/→` adjust LSB/RSB of the active glyph by 1/10/100; `Mod+⌥+←/→` adjust kerning (P2); Escape toggles text/edit mode; typing while in text mode edits the string.
- **Preview strip** (bottom, 96 px, collapsible): the text at three sizes (12/24/48 px) rendered by the kernel-compiled font, with a "stale" indicator (dot + "recompiling…") while the background compile runs; dark/light toggle for the strip only.

## 9. Dialogs

All dialogs: 6 px radius, 24 px padding, title 16/600, body 14, primary button right-aligned, Esc cancels, Enter confirms, keyboard-trapped.
- **New Font** (480 px): family name, style name, units per em (1000 / 2048 / custom), metrics preset (Latin default, CJK 1000 em with 880/−120), starter glyph set (none / Basic Latin / Latin-1 / GB 2312), "Create".
- **Add Glyphs** (560 px): recipe text area with syntax help, charset picker (list with counts and "already present" badges), preview list of names to be added (virtualised), "Add N glyphs".
- **Export** (560 px): format tabs (TTF / WOFF / WOFF2 / UFO / FWF), destination path with "Choose…", options as checkboxes (remove overlap, keep TrueType instructions, preserve untouched tables, production names) with a one-line explanation each, master/instance picker, "Export"; result view lists warnings (`hinting-dropped` etc.) with the affected glyphs as clickable chips and "Reveal in folder".
- **Preferences** (640 px, left categories: General / Editor / Snapping / Shortcuts (P2)): theme, language, autosave interval, nudge steps, hit/snap distances, grid size, tool defaults.
- **Recovery** (on launch): list of recoverable documents with timestamps, "Recover" / "Discard" per row.

## 10. Feedback

- **Toasts** (bottom-right, 320 px, 6 s, stack ≤ 3): success (`--ok` bar), warning, error; error toasts have "Details" expanding the raw message; kernel warning codes map to i18n.
- **Inline audit** (P2): red markers on canvas with tooltips.
- **Busy states**: long operations (export, recompile) show a thin 2 px `--accent` progress line under the tab bar and disable the triggering button; never a modal spinner.
- **Undo labels** in the Edit menu ("Undo Move 3 points"); the status bar flashes the last undo label for 1.5 s.

## 11. Keyboard map (P1)

File: `Mod+N/O/S/⇧S/W`, `Mod+E` export, `Mod+I` Font Info. Edit: `Mod+Z/⇧Z`, `Mod+X/C/V`, `Mod+A`, `Mod+D` duplicate selection (P2), `Delete`/`⇧Delete`. Glyph: `Mod+⇧+G` add glyphs, `Mod+[`/`]` prev/next glyph. Path: `Mod+⇧+O` remove overlap, `Mod+⇧+R` correct direction, `Mod+⇧+E` add extrema, `Mod+⇧+M` simplify, `Mod+⇧+T` tidy, `Mod+J` join, `Mod+B` break (P2). View: `Mod+0/1/=/−`, `Mod+⇧+N` nodes, `Mod+⇧+L` guides, `Mod+⇧+B` background, `Mod+⇧+I` info box, `Mod+⇧+P` palette, `Mod+⇧+F` fill toggle (Space held = temporary). Tools: V P B E F ⇧F L R S Z H T (tap/hold).

## 12. Accessibility and i18n

- Every tool, panel action and field is reachable by keyboard; tool strip buttons have `aria-label` and `aria-pressed`; canvas exposes an offscreen live region announcing selection changes ("3 points selected"); colour is never the only carrier of state (node shapes differ; warnings have icons).
- Contrast ≥ 4.5:1 for all text ≤ 14 px in both themes (measured); ≥ 3:1 for canvas markers against `--cv-bg`.
- Strings via `t()`; Chinese labels verified for width in the tool strip tooltips and inspector headers; numbers use locale-independent formatting (font units are integers or one decimal).

## 13. What "professional" must feel like (acceptance for UI reviews)

- Dragging a node is pixel-exact and 60 fps on a 300-point glyph; the outline never lags the pointer.
- Every hover has a cursor and, where relevant, a ghost; the user can predict the click.
- One gesture = one undo; undo labels are human.
- Panels never jump; opening a glyph does not move the canvas unexpectedly (fit once, then keep the user's zoom per tab).
- Nothing is more than two clicks or one shortcut away; the command palette lists everything with its shortcut.
- Both themes look designed, not inverted.
