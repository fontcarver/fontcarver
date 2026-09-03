<!-- Research report generated 2026-09-03 by an Opus agent from official Glyphs 3 / FontLab 8 / FontCreator 15 / Fontra / RoboFont documentation. The main document comes first; later sections are errata rounds that correct it — where they conflict, the errata win. -->

I have gathered primary-source material directly (full Glyphs handbook text, FontLab 7/8 manual pages, FontCreator 16 docs, FontForge bitmap-view docs, BitFonter/PixelForge, Fontra/RoboFont). Writing the deliverable now.

---

# Cross‑Platform Font Editor: UX & Feature Bar Research

**Scope note on versions (checked September 2026):** Glyphs 4.0 shipped 2026‑07‑27 and `handbook.glyphsapp.com` now documents Glyphs 4; where a behaviour is new in 4 I say so. FontLab is at 8.4.2 (build 8950), but the FontLab 8 manual is unfinished and redirects to the FontLab 7 manual, which is the authoritative text (last updated 2020‑12‑15) — FontLab 8 additions are marked "(unverified)". FontCreator is at 16.0.0.3077 (2026‑07‑17) and is now **Windows *and* macOS**, reads/writes `.glyphs` v3, UFO and designspace. Fontra Pak is at 2026.8.0. RoboFont 4.x.

---

## 1. Application shell & document model

### What the three do

**Glyphs** — exactly one window per font file. A tab strip runs across the top. The leftmost tab is always **Font View** (⌘⌥1); Edit tabs are ⌘⌥2 … ⌘⌥9, ⌃⇥ / ⌃⇧⇥ cycle, ⌥‑click a tab's close button closes all others, ⌘⌥⇧W reopens the last closed tab. An Edit tab is not "a glyph" — it is a **text string** that may contain many glyphs across multiple lines, with one glyph "current" (the one after the text cursor). Edit View has two modes: *text mode* (Text tool, T) and *edit mode* (any other tool, or double‑click, or Esc to toggle).

The right side is the **Palette** (Window → Palette, ⌘⌥P) — a single scrolling stack of collapsible sections: **Dimensions**, **Fit Curve**, **Layers**, **Transformations**, plus a contextual **Stroke** section at the bottom that appears when View → Show Info (⌘⇧I) is on and a path is selected. Plug‑ins add their own sections. The bottom of the Edit window holds a **Preview Area**, toggled by an eye icon at bottom‑left, with a draggable separator; a detachable **Preview Panel** (Window → Preview Panel) mirrors it onto a second display. Everything else is a separate window or sheet: Font Info (⌘I), Kerning (⌘⌥K), Glyph Info, Plugin Manager, Scripting.

**FontLab** — genuinely multi‑surface: a **Font window**, one or more **Glyph windows** (which are modal *within themselves*: Editor / Metrics / Kerning / Elements / Preview, switched by picking the Contour, Metrics, Kerning or Element tool), a **Sketchboard** (free canvas for arranging drawings), and ~30 dockable panels (Anchors and Pins, Brush, Classes, Color, Elements, Features, FontAudit, Font Info, Font Map, Fonts, Gallery, Glyph, Guideline, History, Image, Kerning, Layers and Masters, Metrics, Node, Output, Pairs and Phrases, Preview, Scripting, Source, Stroke, Swatches, Tables, Transform, Variations, View). A **Toolbar** docks to any edge and can be torn off; a context‑sensitive **Property bar** sits under the title bar and changes per tool/mode; there are Rulers, a Grid and a **Scoreboard**. Preferences → General chooses "All fonts and glyphs open in tabs" versus separate windows. Two things worth stealing outright: **hold F1 over any UI item for Quick Help**, and **⇧F1 opens a context‑sensitive Help panel**.

**FontCreator** — historically Windows MDI (child windows inside a parent frame). v16 is cross‑platform with a Font Overview plus glyph edit windows and docked panels (Masters and Layers panel, Validation, comparison/proofing). MDI is the weakest of the three models and I would not copy it.

**RoboFont** is the useful counter‑example: it ships **both** — "Multi Window mode" (Font Overview, Glyph Editor, Space Center, Inspector as separate windows, freely spread over monitors) and "Single Window mode" where the same surfaces become sections of one window.

### Recommendation

**One OS window per open font document, with a tab strip; every tab and every dock panel can be torn off into its own OS window.** Tauri gives us multiple webviews cheaply, so implement tear‑off from day one — type designers routinely run a 32" main display plus a proof display.

- **Tab kinds:** a pinned **Overview** tab (⌘⌥1), N **Edit** tabs (⌘⌥2…9, ⌃⇥ cycles), and *special tabs* rather than modal dialogs for **Features**, **Kerning**, **Font Info**, and **Bitmap Strikes**. Making Font Info a tab instead of a modal window (Glyphs' ⌘I sheet) is a real improvement: designers constantly cross‑reference zones and stems while drawing.
- **Layout:** left dock = context sidebar (filters/categories in Overview; layers + glyph list in Edit). Right dock = **Inspector**, a single scrolling stack of contextual sections (Glyphs' Palette model), *not* a floating palette zoo (FontLab's floating panels are the most common UX complaint about it — unverified but consistent across forum reports). Bottom = collapsible **Preview strip** with a draggable divider.
- **Workspaces:** named, savable, project‑scoped plus global presets ("Drawing", "Spacing", "Proofing", "Hinting", "Pixel"). Restore per‑document on reopen.
- **Command palette (⌘K / Ctrl‑K)** searching every menu command, every filter, every script, every glyph name, and every panel. This is the single highest‑leverage modern affordance none of the three incumbents have.
- **Quick Help on hover (F1)** copied from FontLab, plus a persistent help panel.
- Undo is **per glyph, per layer** (see §11), so the tab model must not imply a single document‑wide undo stack.

---

## 2. Font view (glyph grid / overview)

### Glyphs, concretely

Grid/List toggle at top‑left; **cell‑size slider at bottom‑right**. Large cells draw outline + glyph name + Unicode; small cells draw outlines only. Cell badges are a whole vocabulary worth copying wholesale:

| Badge | Meaning |
|---|---|
| Warning triangle, top‑right | Out‑of‑sync metrics keys |
| Yellow note | Annotation on the active master layer |
| Black info dot | Glyph uses custom glyph properties (deviates from the database) |
| Red top‑left corner | Layers not compatible for interpolation |
| Red ring with stroke | Glyph does not export |
| Lock icon, top‑right | Glyph locked against editing |
| Small system‑font rendering instead of the Unicode indicator | **CJK glyphs** |

Colour labels: **twelve** predefined, addressable by index in scripts and smart filters — 0 Red, 1 Orange, 2 Brown, 3 Yellow, 4 Light Green, 5 Dark Green, 6 Cyan, 7 Blue, 8 Purple, 9 Pink, 10 Light Gray, 11 Dark Gray. There is a *glyph* colour and a separate *layer* colour (⌥ + colour sets the layer colour); the cell paints glyph colour full‑bleed and layer colour on the right half.

**Tags** (Glyphs 3+) are free text; comma or Return commits a tag into a blue capsule; the context menu lists every tag used in the font with checkmark/dash tri‑state for mixed selections. Tags feed smart filters, metric scopes and OpenType feature tokens.

Search field ⌘F searches glyph name, Unicode value, Unicode character or glyph note, with a scope picker, **Match Case**, and **Regex** (e.g. `.*\.sc`, `[Aa].*-cy`).

Left sidebar: **Categories** (with subcategories), **Languages** (scripts and language filters; a "+" edits which scripts are listed; scripts auto‑appear when the font gains glyphs of that script), **Filters** (Smart Filters, gear icon), **Lists**. ⌘‑click multi‑selects within a section; combining a Category with a Language intersects them. Sidebar entries show a badge of *present count / total count*, and right‑clicking one lists the **missing** glyphs with a **Generate** button — an excellent "fill out the character set" affordance.

Smart Filter rules (the full predicate vocabulary, worth matching): Glyph Name, Count of Paths, Count of Components, Tags, Script, Category & Subcategory, Case, Master Compatible, Export Glyph, Has Unicode, Has Components, Contains Component, Has Hints / PostScript Hints / TrueType Hints, Has Corners, Has Anchors, Has Special Layers, Has Custom Glyph Info, Has Annotations, Is Auto‑Aligned.

**Glyph info database.** For each glyph the database defines: nice name, production name, Unicode, components (for composites), a default set of anchors, the marks shown in the **mark cloud**, script, category and subcategory. Suffixed names *inherit*: `A.alt`, `A.ss01`, `A.001` all inherit `A`'s properties, but `Aalt` or `A001` inherit nothing. Script suffixes: `-cy`, `-ar`, `-hb`, `-ko`, `-thai`, `-math`, `-braille`, `-fraktur`, `-musical`. Ligatures join names with `_` (`f_f_l`). Han glyphs have no nice name and use `uniXXXX` / `uXXXXX`. Glyph names must be `[A-Za-z0-9_.-]` and start with a letter; a leading `_` means non‑exporting. Overrides: per‑file in the inspector, or globally via a custom `GlyphData.xml`. `Glyph → Update Glyph Info` re‑derives; Font Info → Other → **Use Custom Naming** turns the whole mechanism off.

Batch: ⌘⇧G *Add Glyphs* accepts names, characters, Unicode values, ranges (`uni01FC:uni01FF`) and **recipes**; ⌘⇧F Find and Replace batch‑renames; Edit → Info for Selection (⌘⌥I) edits properties across a multi‑selection with "Multiple Values" placeholders.

**FontLab** splits this into a **Font window** plus a **Font Map** panel, adds glyph *flags* (marks), *tags* and *classes*, a **Generate Glyphs** dialog with its own recipe syntax, plus **Generate Names** and **Generate Unicodes** actions.

### Recommendation

- **Virtualised, GPU‑drawn grid.** Fonts of 30–60k CJK glyphs must scroll at 120 Hz. Rasterise cells into an atlas on a worker; never re‑tessellate per frame.
- Adopt Glyphs' **badge vocabulary verbatim** — it is learned muscle memory for the target user, and each badge maps to a cheap boolean we already compute.
- **Filters as a saved query language, with a builder.** Expose a small text DSL (`script:han and not export and paths>0`) *and* a visual rule builder that round‑trips to it. This makes filters scriptable, diffable and shareable in the project file — better than Glyphs' opaque predicate sheets.
- **Glyph database as data, not code.** Ship a versioned `glyphdata` file (JSON or TOML) with name→Unicode→category→production‑name→anchors→composition; allow a user file that *overlays* it, and show the black "custom info" badge when a glyph deviates. Also ship a "custom naming" escape hatch for teams with house conventions.
- **CJK specifics:** render CJK cells from a system fallback face when the glyph is empty (Glyphs does this); provide radical/stroke‑count/component sidebar sections in addition to Unicode blocks; support CID mapping and CID‑ordered sort; make the vertical metrics visible in the inspector (top/bottom sidebearing, vertical origin, vertical advance) as a toggle next to the horizontal ones.
- Show a **master switcher** in the toolbar that repaints the whole grid, and an optional "all masters" cell mode that stacks the masters inside one cell (FontLab‑ish, unverified) for compatibility triage.

---

## 3. Glyph edit view — tools & interaction

### Glyphs tools & shortcuts (verified from the handbook)

| Key | Tool / behaviour |
|---|---|
| `V` | **Select**. ⌥‑drag marquee ignores handles; ⌃ while dragging tilts the marquee (italic‑friendly); release ⌃ to resize at that angle |
| `⇧V` | **Lasso** (free‑form selection) |
| `T` | **Text** (text mode). Esc toggles text/edit |
| `P` | **Draw** (pen). Click = node; drag = curve; ⌥ while dragging changes only the outgoing handle; ⌘ while dragging changes only the outgoing handle *length* keeping both angles; Space repositions the node mid‑drag; click first node closes path; **click an existing node opens the path there**; **⇧‑click a segment inserts a node at the nearest extremum or inflection** |
| `B` | **Pencil** (freehand) |
| `F` / `⇧F` | **Primitives** — rectangle / ellipse. Click once = numeric entry dialog; ⇧ = square/circle; ⌥ = draw from centre |
| `E` / `⇧E` | **Knife** / **Erase** (shared icon; ⇧+key cycles tools sharing an icon) |
| `R` | **Rotate** — click sets origin, then drag |
| `S` | **Scale** — same; ⇧ constrains |
| `L` | **Measurement** |
| `Z` | **Zoom** (⌥‑click zooms out); ⌘Space / ⌘⌥Space temporarily |
| `H` / Space | **Hand** |
| `A` | **Annotation** (Info box becomes a palette: select, text, arrow, circle, plus, minus) |
| `X` (or ⇧B from Pencil) | **Pixel tool** — see §9 |
| `I` | **Higher‑Order Interpolation** (Glyphs 4) |
| — | **TrueType Instructor** (its own modifier‑free shortcut set, see §8) |

Node semantics — this is where "professional feel" lives:

- Smooth nodes are **round and green**, corner nodes **square and blue**; handles are small circles joined by thin grey lines. Open‑path ends draw as short perpendicular blue lines; the start node draws a blue triangle showing path direction. Node size/colour is user‑configurable, per light and dark appearance.
- **Double‑click a node, or select and press Return, toggles smooth ↔ corner.** ⌥ while doing so preserves the existing outline instead of levelling the handles.
- **⌥⇧Return (or ⇧double‑click) toggles "super‑smooth" (G2) nodes** — between two curves a *star* icon enforcing curvature continuity, with the original path drawn grey behind and a small grey dot marking the effective node; between a line and a curve a *raindrop* icon that makes squircle‑style transitions, with grey dots showing the space the transition needs.
- Moving: arrows move 1; ⇧ = 10; ⌘ = 100 (both configurable in Font Info → Other → Keyboard Increments). ⌥ moves nodes *without* their handles. **⌃⌥ while dragging nudges** — proportionally adjusts the surrounding unselected handles. ⌥‑dragging a handle preserves its angle. **⌃⌥ on a smooth node's handle mirrors length and angle to the opposite handle.** Dragging a segment moves both nodes and handles; ⌥‑dragging a segment changes handle lengths but keeps angles. ⌥‑clicking a line segment converts it to a curve; deleting a handle converts back.
- Delete keeps the path closed and reconstructs the segment; **⇧Delete** removes the node *and* both adjacent segments; ⌥ variants apply to all masters.
- Tab / ⇧Tab step to the next/previous point on the path — also works for guides.
- **Nodes on a vertical metric draw a beige diamond; nodes inside an alignment zone draw a beige circle.** Duplicate coincident nodes draw a red circle.
- **Open Corner / Reconnect Nodes** split a corner into two nodes with a triangular overlap sized to ~half the first stem value from Font Info; small overlaps are treated as "invisible". This is the single best interpolation‑friendliness feature in Glyphs.
- **Focus on Selected Path** hides controls for every other path; **Lock Path** makes points unselectable (they turn red on hover); **Locked** on a whole glyph shows a lock badge and blocks filters and scripts too.
- Commands: ⌘⇧T Tidy up Paths (⌘⌥⇧T all masters), ⌘⇧R Correct Path Direction (⌘⌥⇧R all masters; also normalises start nodes and shape order), ⌘⇧A Align Selection, Path → Add Extremes (⌥ → *Force Extremes*), ⌘⌥⇧B bounding box, ⌘⇧I Info box, ⌘⇧N show nodes, ⌘⇧L show guides, ⌘⇧B show background, ⌘B edit background, ⌘J selection→background, ⌘⌥J add to background, ⌘⌃J swap with background.

**Guides.** Local (blue, layer‑scoped) vs global (red, master‑wide); ⌥ + context menu adds a global guide; guides can be locked (lock icon replaces the knob), named (name shows at the left end), angle‑locked via a lock in the Info box, and positioned relative to LSB, RSB or both. **A guide can be turned into a *measurement guide*** which permanently displays distances at its intersections with the outline. A global guide can be scoped to a glyph subset using the same predicate language as Smart Filters. Pro move: with the Measurement tool, hold ⌘⌥⌃ and press `G` mid‑drag to drop a guide along the measurement line.

**Snapping.** Objects snap to nodes and handles on paths *and inside components* — when you approach a component, Glyphs fades in ghost representations of the component's interior nodes. **Hold ⌃ to disable node snapping.** Background objects participate in snapping when the background is shown. FontLab formalises this better: a configurable **hit‑to distance** (default 3 screen px for nodes, 4 for guidelines) and **snap‑to distance** (default 5 px), with independent toggles for Snap → Outline (⌥⌘'), Guides (⌘F9), Font Guides (⇧⌘F9), Grid (⇧⌘').

**Info box (⌘⇧I).** Glyph name centred at top, Unicode at top‑right (with a button that hands the code point to UnicodeChecker). For a single node: X/Y. For a handle: ΔX, ΔY and handle length. For a multi‑selection: X, Y, width, height, a nine‑point transformation origin, and a proportional lock; solid square = selected point count, outlined square = total points. For a component: name, X/Y offset, horizontal/vertical scale %, slant, rotation, plus an arrow button that inserts the base glyph into the tab. For a guide: position, angle (lockable), name, LSB/RSB anchoring, measurement toggle. Values are editable in place; Tab / ⇧Tab walk fields; ↑/↓ increment (⇧ = 10).

### FontLab tools & shortcuts (verified)

FontLab's defining interaction is **tap vs hold**: tap a letter to switch tool permanently, **hold** it to borrow the tool until you release. You can live in the Contour tool and momentarily hold `J` for the Knife, `2` for the Eraser, `S` to scale, `R` to rotate.

| Key | Tool |
|---|---|
| `A` / `1` | Contour (also temporarily via holding ⌘ if enabled) |
| `V` | Element (moves whole elements/components) |
| `M` | Metrics mode · `K` Kerning mode · `T` Text (tap only) |
| `2` | Eraser (⇧ removes any node, not just non‑extrema) |
| `B` Brush · `N` Pencil · `5` Rapid · `P` Pen | drawing |
| `J` / `3` | Knife (add nodes, slice) · `Q` Scissors |
| `U` | Magnet (link nodes to guides → Power Guides) |
| `F` Fill (bucket) · `G`/`4` Guides · `O` Ellipse · `I` Rectangle | |
| `W` / `E` | Flip horizontal / vertical (tap only) |
| `S` / `D` / `R` | Quick Scale / Quick Slant / Quick Rotate (hold only) |
| `C` | Power Nudge (hold; ⇧C latches) |
| `Z`/`Y` zoom in, `X` zoom out, `` ` `` Quick Preview (hold), Space Pan & Preview (hold) | |
| `L` Tunni lines · `H` Preview Rounding (live integer rounding) | |
| `/` glyph‑name input · `#` Unicode input · `[` `\|` `]` align left/centre/right | |

(The Toolbar page and the Tools page disagree — the Toolbar page lists Eraser as `Q` and Scissors as `K`; the Tools page, which is more detailed, lists Eraser `2` and Scissors `Q`. Treat the Tools page as authoritative.)

Menu shortcuts worth noting: ⌘T Free Transform, ⌥⌘L Edit Tunni Lines, ⌥⌘A Add Smart Corner, ⌥⌘B Simplify, ⌥⌘J Nodes at Extremes, ⌘F10 Remove Overlap, ⇧⌘O Make Overlap, ⇧⌘B Unlink Corners, ⌘B Break, ⌘J Join, ⇧⌘3 / ⇧⌘2 To PS / To TT curves, ⌥⌘X / ⌥⌘Y X‑Servant / Y‑Servant, ⇧⌘W Add Power Guides, ⌘M Copy Outline to Mask, ⌃H Edit Mask, F7 Autohint, ⇧⌘F7 TrueType Hinting, ⇧⌘F FontAudit, ⌘5 Zoom 1:1 px.

FontLab‑specific concepts, precisely:

- **Genius nodes** — a PostScript node that permanently maintains **G2 continuity**: the node's own position is recomputed from wherever you put the handles, so moving a handle can move the node. Drawn with a circle around the node symbol. Equivalent to "permanently Harmonized". Glyphs' super‑smooth star node is the same idea with a different UI.
- **Servant nodes** — X‑Servant / Y‑Servant: the node's X (or Y) is *interpolated* between neighbouring non‑servant nodes; move the rightmost node by 100 and an X‑servant midpoint moves by 50, handles scaling appropriately.
- **Smart corners** — select a corner node (or a contour crossing with no node) → Add Smart Corner. A blue handle appears; drag outward for a rounded corner, inward for an **ink trap**; or type a radius in the `R` field of the Node panel (positive = round, negative = trap). ⌥ prioritises the handle over the node when they're close. Multi‑select and drag one handle to adjust all. Stored as a live effect in `.vfc`/`.vfj`; baked to curves on export.
- **Power Guides** — guidelines, metrics or alignment zones **linked to nodes**. Drag the guide, the linked nodes come with it, preserving distances. Created automatically for all H/V stems via Glyph → Add Power Guides (⇧⌘W), or manually by dragging with the **Magnet tool** from a node to a guide/zone/metric. Links draw as green dashed lines with an arrow; View → Show → Node Links toggles them. Preferences choose which guide classes are eligible (element only → element+glyph → +font → +zones → +metrics). This is the cleanest "rescale a glyph while preserving stem widths" mechanism in any editor.
- **FontAudit** — always‑on outline analysis. Errors draw as red arrow pointers in the Glyph window and red corner marks on Font‑window cells; hover shows the short name, click opens details with **Fix** and **Fix All**. Tests include Curvature (node near a smooth‑curvature point), Disproportional (unbalanced control points), Empty lines/curves, Empty contours, Open contour, Vectors on ClosePaths, Flat curves, Collinear vectors, Inflections on curves, "Weak" extremum points, and more.
- **Elements & element references** — FontLab's component model is one level more general than Glyphs': an *element* is a reusable shape container; a glyph is a stack of elements; an *element reference* is an instance. Elements carry **stickers** (⌃T) and can be locked (⌃L), grouped (⌘G), arranged front/back (⇧⌘] etc.), and "Place As Glyphs" (⌃P).
- Also: **Tunni lines** (`L`) for direct curve‑tension manipulation, **Power Nudge** (`C`), **Power Brush** (⇧⌃B), **Rewind Record / Rewind / Fast Forward** (⌃A/⌃V/⌃B) for replaying edits, **Glue Selection** (⇧⌘A), **Create Parallel Contour**, **Expand Contour**, **Slanted Grid**, and PS↔TT curve editing in the same window with explicit conversion commands.

**FontCreator 16** offers point/contour editing on both quadratic and cubic curves (choosing the export flavour later), a Transform toolbar (rotate/scale/skew/slant plus "effects" like bold/italic/small‑caps derivation), the **Glyph Transformer** for scripted transform chains, real‑time **Validation** with auto‑fix, **kink detection** across masters, **Path Decorations** (v16 — designed corner and cap shapes, i.e. Glyphs' `_corner`/`_cap` by another name) and **Smart Components** (v16 — parameterised glyph parts).

### Recommendation for us

Adopt Glyphs' node grammar (it is the de‑facto standard our target user knows) and FontLab's *tool ergonomics*:

1. **Tap/hold tool switching** for every tool. This is the biggest single ergonomic win available and it is cheap to implement.
2. **Modifier grammar, fixed and documented:** ⇧ = constrain/×10, ⌘ (Ctrl) = ×100, ⌥ (Alt) = "the other behaviour" (ignore handles, from‑centre, duplicate, all‑masters), ⌃ = suspend snapping. Never overload ⌥ for two things on the same tool.
3. **Live HUD near the cursor** while dragging: delta X/Y, angle, segment length, resulting stem width when two nodes are selected. Glyphs puts this in a fixed Info box; a cursor‑following HUD plus the fixed box is better.
4. **Cursor changes on hover** must encode intent before the click: over a segment = "insert node" (with a ghost node at the projected position), over a node = move, over a node with ⌥ = break handle, near a component = ghost component nodes fade in, over a locked path = red points.
5. **Both G2 flavours:** a permanent G2 node (Glyphs star / FontLab Genius) *and* a one‑shot Harmonize command.
6. **Live corner/cap decorations** as first‑class non‑destructive effects with a numeric radius field and a drag handle (FontLab Smart Corners) *plus* reusable component‑based corners (Glyphs `_corner.*`). Ship both; they solve different problems.
7. **Node‑linked guides** (Power Guides) — a genuine differentiator, and straightforward with a constraint solver.
8. **Always‑on outline audit** with inline red markers, hover tooltip, Fix / Fix All, and a per‑test enable list. Run it incrementally on a worker.
9. **Undo granularity:** one undo entry per gesture, not per mouse‑move. A drag of 40 nodes is one entry; typing a value in the Info box is one entry; a filter application is one entry. Coalesce arrow‑key nudges within 500 ms into a single entry.

---

## 4. Spacing & kerning

### Glyphs

**Metrics keys.** LSB, RSB and width fields accept numbers *or* keys:

| Syntax | Meaning |
|---|---|
| `n` or `=n` | Take the corresponding sidebearing from glyph `n` |
| `=n+10`, `=n-10` | Arithmetic on it |
| `=\|n` | **Mirrored** — take the *opposite* sidebearing of `n` |
| `=50` | Constant |
| `==m` | **Local** — applies only to the current master |
| `=A@200` | Measure `A` **200 units above the baseline** |
| `=+25`, `=-10` | Offset an *automatically aligned* metric while keeping it automatic |

Out‑of‑sync keys show **red with an update button** in Edit View and a warning triangle on the Font‑View cell. `Glyph → Update Metrics` = **⌘⌃M**; ⌥ variant `⌘⌥⌃M` updates all masters.

**Spacing shortcuts** (Text tool active): `⌃←/→` LSB, `⌘←/→` RSB, `⌘⌃←/→` both; add ⇧ for steps of 10. (Note the macOS Mission Control conflict on ⌃←/→ — we must not depend on ⌃+arrows as our only binding.)

**Kerning modes** — a bottom‑right icon cycles **no kerning / kerning / locked kerning**. "Locked kerning" previews kerning but **disables the spacing shortcuts**, so you cannot accidentally respace while kerning. Copy this.

**Kerning shortcuts** — mnemonic: Control is on the left of the keyboard, Command on the right.

| Shortcut | Action |
|---|---|
| `⌥⌃←` / `⌥⌃→` | increase / decrease kerning on the **left** side |
| `⌥⌃⇧←` / `⌥⌃⇧→` | same, in steps of 10 |
| `⌘⌥→` / `⌘⌥←` | increase / decrease kerning on the **right** side |
| `⌘⌥⇧→` / `⌘⌥⇧←` | same, in steps of 10 |

**Groups & exceptions.** Each glyph has a left and a right kerning group (top/bottom for vertical scripts), edited in the Info box or the Font‑View inspector; names allow `[A-Za-z0-9_.-]`. Kerning is group‑to‑group by default. **An exception is created by opening the padlock next to one side in the Info box** — leaving the other side locked keeps the exception applying across that side's group. `View → Show Group Members` paints all members of the relevant group behind the current pair, which is how you catch collisions. In Edit View, **negative kerning renders light blue and positive kerning yellow** (both configurable).

**Kerning window (⌘⌥K):** columns Left / Value / Right, sortable; groups render blue with a leading `@`, single glyphs gold; search with contains / is exactly / begins with / ends with / does not contain, plus filters for "is a group", side, and "only glyphs selected in Font View". Selecting a row shows the pair in Edit View; ↑/↓ walk the whole kerning table. Renaming a group in the table asks whether to rename just this pair or font‑wide. **Clean Up** deletes obsolete pairs; **Compress** converts singleton pairs into group kerning wherever possible, keeping true exceptions. Copy/paste kerning between masters with overwrite/keep‑existing conflict resolution.

Glyphs 4 adds **visual kerning group building**: drag glyphs onto a shelf to form groups instead of typing group names.

### FontLab

Metrics mode (`M`) and Kerning mode (`K`) are *modes of the Glyph window*, each with its own Property bar and (optionally) its own background colour.

Metrics editing: `;` auto‑calculates metrics for the active glyph. `⌃←/→` LSB ±1 (⇧ = 10), `⌥←/→` RSB, `⌃⌥←/→` shifts the glyph within its advance width, `-`/`+` (or `⌃⌥↑/↓`) adjusts both sidebearings. ⌘ makes it 100; **⇧⌘ forces a *true* 1 unit regardless of the configured step**. `[`/`]` or ←/→ walk glyphs in the text; ↑/↓ walk lines or **phrases from the Pairs and Phrases panel**; `,`/`.` replace the active glyph with the previous/next glyph in the font; `/` enters a glyph by name. Sidebearing lines are draggable, as is the glyph inside its advance (⌥‑drag changes the width). Values accept **expressions** (`(20+15)/2`) which are *kept as expressions*, plus metric links across glyphs and across masters, and "binding sidebearings".

Kerning: class‑to‑class is the default and there is **no "key glyph"** — all class members are equal. Drag the second glyph, or the "thumb" below the pair; **green bar/thumb = negative kerning, yellow = positive, white = none**. ←/→ = 1 unit, ⇧ = 10, ⌘ = 100. **Hold ⌃ to drop the first class, ⌥ to drop the second, ⌃⌥ to kern the two individual glyphs** — this is a far better exception model than Glyphs' padlocks because it requires no state change. `W` swaps the two glyphs of the pair. The **Kerning Cloud** overlays every member of a class behind the pair (Glyphs' Show Group Members). A **Metrics table** gives a spreadsheet of W/LSB/RSB/kern.

**FontCreator**: Tools → **AutoMetrics** wizard with an **Optical Metrics** option that measures actual glyph shapes and derives balanced sidebearings, governed by a single global **glyph spacing factor**; plus autokern and a class‑based kerning UI. Their own tutorial admits optical metrics struggles with bowl combinations (`be`, `pe`, `po`) in serif faces.

**RoboFont Space Center**: a dedicated window with a text field, a **Space Matrix** at the bottom accepting numbers, references and calculations, a **beam** to measure margins at an arbitrary horizontal cross‑section, and a **Cut Off** display mode that clips glyphs to make counters legible.

### Recommended keyboard model for us

Do **not** inherit `⌃+arrow` (macOS Mission Control) or `⌥+arrow` (word navigation in text fields) as the only path. Proposal, identical on macOS and Windows/Linux with ⌘↔Ctrl swapped:

| Binding | Action |
|---|---|
| `Alt ←/→` | LSB −/+ 1 |
| `Alt ⇧←/→` | LSB ∓10 |
| `Ctrl/⌘ ←/→` | RSB −/+ 1 (⇧ = 10) |
| `Alt Ctrl ←/→` | both sidebearings (shift glyph in width) |
| `Alt ↑/↓` | advance width ∓1 |
| `,` / `.` | kern current pair by −1 / +1 (⇧ = 10, `Alt` = 100) |
| `Alt ,` / `Alt .` | kern the *previous* pair |
| `0` (zero) | delete the current kern pair |
| `Ctrl` held while kerning | ignore the left class · `Alt` held | ignore the right class · both = pure glyph pair |
| `;` | auto‑space the current glyph |
| `L` | toggle spacing‑lock mode (kern without risk of respacing) |

Comma/period for kerning is a deliberate break from both apps: it is reachable with the right hand while the left hand holds class modifiers, works identically on every OS, and leaves the arrow keys for node editing. Ship a "Glyphs" and a "FontLab" keymap preset alongside our default, importable/exportable as a file (FontLab already lets users import settings and shortcuts).

Also required: a Space‑Center‑equivalent **spacing tab** (string + editable metrics matrix + beam + cut‑off), **Pairs and Phrases** style saved kerning strings, an autokern/auto‑space engine (optical, with a global density factor), and kerning **Clean Up** / **Compress** as first‑class commands.

---

## 5. OpenType features

**Glyphs.** Font Info → Features, sidebar split into three sections with individual "+" buttons: **Prefix** (arbitrary code outside features, typically standalone lookups), **Classes** (whitespace‑separated glyph name lists), **Features** (picked by tag or name from a searchable list; a **Spec** button opens the Microsoft registry page for the selected tag). A single **Update** button (bottom‑left) regenerates *all* automatic prefixes, classes and features, and discovers new ones — add `f_l` to the font and `liga` appears. Each auto entry has its own small update button; unchecking **Generate Feature Automatically** converts it to manual code, which Update then leaves alone. Code is AFDKO `.fea` with syntax highlighting, autocomplete after `sub`/`lookup` keywords, ⌥‑click on a glyph or class name for a **visual preview** (class members listed), a **snippets** button (extensible via `FeatureSnippetsNAME.plist`), and per‑entry notes. Errors appear **inline** with an expandable description; errors carrying a **wrench** icon offer a **Fix** button; the sidebar marks failing entries and clicking the marker jumps to the offending line. Stylistic sets get localizable feature names with a language picker per row.

Feature generation is name‑driven: `liga` from underscore names (with a hardcoded list — `f_f_i`, `f_f_l`, `f_f`, `fi`, `fl`, plus Thai/Cyrillic cases — going to `liga` and everything else to `dlig` unless suffixed `.liga`), `smcp`/`c2sc` from `.sc`/`.smcp`/`.c2sc`, `hlig` from `longs` or `.hlig`/`.hist`, `rlig` from `.rlig` plus the Arabic lam‑alef set, `aalt` synthesised from every substitution feature, and generally `x.ss01` → `ss01`. Dot suffixes chain and **must be ordered to match the feature order in the Features tab** (`x.sc.ss01`). Kerning, mark, mkmk and curs are **implicit** — generated from kerning values and anchors, never shown as editable code. Glyphs also has non‑standard extensions: number‑value tokens, glyph‑property tokens, glyph‑predicate tokens, `#ifdef VARIABLE` conditional blocks, and variable GPOS.

**FontLab** has a Features panel (code editor + compile + errors) backed by a Classes panel; automatic generation is weaker (unverified extent).

**FontCreator 16** is the outlier and the most interesting: a visual **OpenType Layout Designer** with an interactive proofing view, **plus** a code editor supporting both their OTLFD syntax and standard FEA, with "a fully compliant Adobe fea compiler" included. Their headline claim is that they can *modify an existing font while preserving all its original lookups, lookup flags and feature parameters* — which is only possible with a genuinely round‑trippable structured model.

### Recommendation

**One source of truth: the structured lookup model. FEA text is a bidirectional projection of it.**

- Parse FEA into a typed IR (features → lookups → rules), and render it back. Round‑trip fidelity (comments, ordering, lookup flags, feature params) is the hard part and the reason to invest early — FontCreator's differentiator proves the market value.
- **Split view:** left = feature/lookup tree; right = either a **rule table** (visual editor for the lookup types we understand: GSUB 1/2/3/4/6, GPOS 1/2/4/6) or the **FEA text** for that node, switchable per node. Nodes we cannot represent visually show read‑only structure plus editable code, never a silent failure.
- **Auto‑generation as a separate, visible layer.** Each feature/class is either `auto` (regenerated from glyph names, with a per‑entry refresh button and a global Update) or `manual`. Show the generated code read‑only with a "Detach to manual" action. Do not merge generated and hand code in one buffer.
- **Inline diagnostics with fixes.** Errors and warnings in the gutter, wrench icon where an automatic fix exists, click‑to‑jump from the tree.
- **Live shaping preview attached to the editor** (see §10) so a rule can be verified in the same view where it is written, with the sample string editable per feature.
- Editor affordances: autocomplete for glyph names/classes/lookups, ⌥‑click to preview a class's members as glyph cells, snippets library (user‑extensible), a "Spec" link per tag.

---

## 6. Font Info

**Glyphs 4** — ⌘I, tabs: **Font**, **Masters**, **Exports**, **Features**, **Other Settings**, **Notes**. The Font tab is a **properties list** with `+` / `−` buttons rather than a fixed form, so you add only the fields you need, each with a language menu for localised values; rows support ⌘‑click multi‑select and ⌘C/⌘V. Axes are defined here. **Custom parameters** are the extension mechanism at every scope (font / master / instance / variable‑font‑setting) — arbitrary typed key/value pairs that drive export behaviour (`Remove Glyphs`, `Rename Glyphs`, `Export Folder`, `Virtual Master`, `Feature for Feature Variations`, `TTFAutohint options`, `Write lastChange`, `Enforce Compatibility Check`, `Disable Masters`, …). Masters carry: axis coordinates, vertical metrics (ascender, cap height, x‑height, descender, italic angle), **alignment zones**, **stems**, **number values**, and their own custom parameters. Exports carry: active flag, style name, axis coordinates, style linking, custom parameters. Other Settings holds **Grid Spacing & Subdivision**, **Keyboard Increments** (the ⇧ and ⌘ step values), Use Custom Naming, Disable Automatic Alignment, Keep Alternates Next to Base Glyph, File Format Version, and **Font Type** (classic MM interpolation vs "variable interpolation" for static instances). Notes is a Markdown‑highlighted text field.

**FontLab** splits Font Info into **Master Properties** and **Font Properties**. Master: Names (family, style, full, PostScript, style group, style link, **Build Names** button, master name), Family Dimensions (UPM, ascender, descender, line gap, safe top/bottom, bbox top/bottom), Font Dimensions (caps height, x‑height, italic angle, slope, caret offset, **curve tension**, **corner tension**, **ink‑trap width**, underline position/thickness), Stems, Zones, Parameters, Other Values (sub/superscript, strikeout), PostScript Hinting, TrueType Hinting and Smoothing. Font: Creator (vendor code/name/URL, designer, designer URL, creation date, version, description), Legal (copyright, trademark, license URL, license), Embedding, Codepages, Unicode Ranges, Axes, Masters, Instances. Separately there is a **Font Info *panel*** which shows a subset and can **compare properties across two or more open fonts** — very useful for family QA.

**FontCreator** uses a Font Properties dialog with tabs (Identification, Metrics, Ranges, Embedding, Classification, Layout, Advanced) (unverified exact tab set for v16).

### Recommendation

Three explicit **scopes**, visible as a breadcrumb in every field: **Family → Font/Source → Master → Export**. A value inherits down and shows the inherited value greyed until overridden, with a clear "override / revert to inherited" affordance. Concretely:

- Tabs: **Names & Legal**, **Axes**, **Masters** (metrics, zones, stems, per‑master parameters), **Exports/Instances**, **Features**, **Settings**, **Notes**.
- A **flat searchable view** across every field ("find `panose`", "find `usWeightClass`") — the single biggest quality‑of‑life gap in all three incumbents.
- Custom parameters reimagined as **typed, scoped, discoverable settings**: a schema‑backed list with descriptions and validation, so `+` shows a searchable catalogue instead of requiring the user to know the magic string. Keep an "unknown key" escape hatch for forward compatibility.
- Inline validation and a preflight panel: vertical‑metric consistency across masters, name‑table length limits, style‑linking sanity, missing `.notdef`, illegal glyph names.
- A **Compare Fonts** mode (FontLab's Font Info panel + Glyphs' Compare Fonts / Compare Family) diffing two open sources field by field.

---

## 7. Variable fonts

**Glyphs.** Axes in Font Info → Font; masters place themselves at axis coordinates; **Exports** holds both static instances and **Variable Font Settings** (multiple, each producing a separate VF, each able to carry filters, `Remove Glyphs`, `Add Feature` parameters, and `Disable Masters` to **subset the designspace** — axes with no remaining masters vanish).

Per‑glyph designspace surgery uses two layer kinds:
- **Intermediate ("brace") layers** — named `{90}` or `{80, 120}`; add a layer to the nearest master and mark it Intermediate, then type axis coordinates. Empty fields inherit from the host master.
- **Alternate ("bracket") layers** — named `[90‹wg]` or `[80‹wg, 25‹wd‹50]`, holding min/max axis ranges. They need not be compatible with the master layers, only with each other. In VFs they are activated by a feature named in the `Feature for Feature Variations` parameter (default `rlig`; `rvrn` common). For static instances the equivalent is `Rename Glyphs` + `Remove Glyphs`. A layer can be both brace and bracket.
- **Virtual masters** — an axis plus a `Virtual Master` custom parameter, then Intermediate layers only on the handful of glyphs the axis affects (e.g. crossbar height on `A E F H`). No redrawing of the rest of the font, no kerning duplication, and only those glyphs get deltas.

**Compatibility.** Compatible = same path count and node order, same anchor count and names, same component count and referenced glyphs, same shape order. Incompatible glyphs draw a red bar above the ascender in Edit View and a red corner in Font View, and there's a built‑in "Incompatible masters" Smart Filter. `⌘⇧R` Correct Path Direction fixes direction, normalises start nodes and reorders shapes (⌘⌥⇧R = all masters); Filter → Shape Order gives manual reordering across all layers.

**Show Master Compatibility (⌘⌥⌃N)** is the best diagnostic UI in any editor: paths and components coloured by shape order (components additionally checkered), shape‑order index numbers next to each start node, diagonal lines connecting start nodes / shape centres / anchors across masters (anchors dashed), and segments coloured **green** (compatible), **yellow** (compatible but the angle differs by more than 20° between masters, i.e. a node is probably missing) or **red** (incompatible). **Crossing diagonals reveal "shape shifters"** — compatible but mismatched shapes — and you fix them by *dragging a point on a connecting line onto another line to swap the shapes*.

Glyphs 4 adds an **Axes sidebar with live interpolation preview inside Edit View**, and the **Higher‑Order Interpolation tool (`I`)**: nodes' layer positions draw as diamonds (yellow = next layer down the axis, blue = current, purple = next up), the default linear path draws as a grey dotted line with a knob in the middle; drag the knob to bend the interpolation path, press Return to convert the intermediate point into a pair of handles, or switch it to **Rotation** around a centre point with Automatic / Clockwise / Counterclockwise direction. This applies to static *and* variable export.

**FontLab** has a **Variations panel** (axes, masters, instances) and the **Matchmaker tool**: a schematic mode where contours and nodes are drawn abstractly, a status circle in the Property bar goes green/red, a blue handle at the zero point rotates the "angle of view" across masters, and you fix a segment by marquee‑selecting the matching nodes at one end (blue group), then the other end (orange group), then clicking a group — FontLab inserts (or, with "Allow to clean up segments", removes) nodes to make the segment match everywhere. `Glyph → Match Masters` (⌃M) attempts the whole glyph; "Clean Up All Masters" runs Clean Up everywhere. It also warns that same‑node‑count incompatibility usually means **mixed PS and TT segments**.

**Fontra** contributes the most forward‑looking idea: **variable components** — a component that carries its own axes and is instanced with per‑use axis values, plus sparse sources. This is Glyphs' Smart Components generalised, and it is the correct model for CJK (a radical with width/height/stroke‑weight axes, reused thousands of times).

### Minimum viable set for us (v1)

1. Axes (tag, name, min/default/max, hidden flag), masters at coordinates, instances at coordinates, and at least one VF export config with designspace subsetting.
2. Compatibility checking with the Glyphs colour vocabulary (green/yellow>20°/red), shape‑order indices, cross‑master connector lines, **and drag‑to‑swap repair**.
3. Correct Path Direction / normalise start node / reorder shapes, all‑masters variants.
4. **Intermediate (brace) layers** and **Alternate (bracket) layers with a proper condition editor** (not a name‑encoding scheme — store ranges as data, display the `[90‹wg]` string as a label).
5. Live interpolation preview: an axis slider strip in Edit View that scrubs the drawing continuously, plus a preview strip showing named instances.
6. Sparse/virtual masters.
7. **Variable components with their own axes** (Smart Components), because it is the CJK story and it is also how we make pixel fonts cheap (§9).
8. Defer: higher‑order interpolation, avar2/axis mappings beyond a simple mapping table, STAT authoring beyond defaults.

---

## 8. Hinting & export

**Glyphs PostScript hinting:** font‑wide **Standard Stems** (multiple values allowed, with the caveat that two stems 70 and 80 may render 2 px and 3 px at the same size) and **Alignment Zones** per master, with `TTFBlueFuzz`‑style tolerances; glyph‑level stem hints, ghost hints and flex hints; autohinting at export. **TrueType hinting** has a dedicated **TrueType Instructor tool** whose shortcuts deliberately have **no modifiers** (so `A` adds a Snap instruction rather than switching to Annotation): `A` Snap, `S` Stem, `F` Shift, `G` Interpolate, `E` Delta. Three outlines are drawn simultaneously — **black** = current master (nodes as grey discs, extra nodes always shown), **blue** = current instance before hinting, **red** = after hinting and grid‑fitting for the current rendering intent (Grayscale / ClearType / DirectWrite) at the current pixel size; a pixel preview sits behind, toggled by "Show Pixel Preview". The Info box gains a **pixel size (PPM)** field with steppers and arrow‑key editing. Font‑level TrueType zones and stems have name/position/size/alignment/deltas/filter. ttfautohint is available via a `TTFAutohint options` custom parameter with the full option set (blue fuzz, x‑height rounding, fallback script, `.ttfautohint` ghost component, detailed info in name ID 5, etc.). Per‑glyph autohint is `⌘⌥⌃⇧I`.

**Glyphs export (⌘E; ⌘⌥E exports all open files silently):** tabs **OTF**, **Variable Fonts**, **UFO**, **Metrics**. OTF tab: outline flavour (PostScript/CFF `.otf` vs TrueType `.ttf` — components survive in TT flavour), file formats (`.otf`/`.ttf`, `.woff`, `.woff2`, multi‑select), options **Remove Overlap** and **Autohint** (PS: applied only to glyphs without manual hints; TT: applied font‑wide, *ignoring* manual TT hints), destination folder checkbox + path, and **Test Install** which writes directly into the macOS font system (visible to Core Text apps, not Adobe/Affinity, and lost on reboot). Per‑instance `Export Folder` parameters can nest or escape the destination. Variable Fonts are always TrueType flavour. UFO export offers "Convert Glyph Names to Production Names" and "Decompose Smart Stuff". Metrics export writes a proprietary `.metrics` file or AFM.

**FontLab export profiles** — named, duplicable presets under File → Profiles. Defaults: OpenType TT (.ttf), OpenType PS (.otf), FontLab (.vfc), FontLab JSON (.vfj), UFO Package, FontLab Classic (.vfb), PostScript Type 1 (.pfb), DesignSpace+UFO, OpenType Variations (.ttf), Web TT (.ttf/.eot/.woff), Web PS (.woff2/.woff). Editable parameters include profile name, **profile subfolder**, format, curve conversion + conversion tolerance, correct contour direction, remove overlaps, nodes at X extremes, nodes at Y extremes, clean up, round coordinates to UPM grid, and more. Default profiles are read‑only; you duplicate to customise.

### Recommended export dialog

Steal FontLab's **profiles** and make them project data:

- A profile is a named, versioned object **stored in the project file** (so it is in git and shared with the team), with an optional user‑global library. Each profile: output format, flavour, subfolder, and an explicit option set (remove overlap, decompose, autohint, subroutinize/CFF optimisation, curve conversion tolerance, extrema insertion, coordinate rounding, production names, subsetting, `.woff2` compression level).
- The export dialog is a **matrix**: rows = instances / variable‑font settings, columns = profiles, checkboxes at intersections, with "select all", saved export sets, and a running estimate of output file count.
- **Preflight before writing anything**: illegal glyph names, incompatible masters, unresolved components, feature compile errors, vertical‑metric mismatches, kerning referencing deleted groups. Show it as a list with click‑to‑jump, exactly like Glyphs' export error dialog (which has a "Show" button opening the offending glyph in a new tab).
- **Test install** on all three platforms (macOS Core Text, Windows `AddFontResourceEx`, Linux `~/.local/share/fonts` + `fc-cache`), plus "copy to Adobe fonts folder" and "open in browser preview".
- **The CLI uses the same profile file.** `fontworkshop export --profile web --instances all` in CI must produce byte‑identical output to the GUI. This is a genuine differentiator: Glyphs needs a plugin/script, FontLab needs a macro.
- Hinting: ship ttfautohint (or the Fontations autohinter port) with a proper options UI, PS autohinting with stems/zones from Font Info, and defer manual TT instruction editing to a later tier — but design the data model to hold instructions from day one.

---

## 9. Bitmap / pixel font editing

### The state of the art

**FontForge** is the only mainstream outline editor with a real bitmap view. Element → **Bitmap Strikes Available…** lists pixel sizes; adding a size creates that strike in the SFD, removing one deletes it. Element → **BDF Info** edits BDF properties. The **Bitmap Glyph View** has: a small unmagnified preview of the glyph at top; a readout of the pointer's pixel coordinates (origin at the glyph origin); a second readout of the drag delta while the button is down; and a button that **re‑rasterises the outline into the editable bitmap**. Three layers — bitmap (always the editable one), outline, grid — with independent visibility. Six palette tools: **pointer** (marquee select, move/copy the selection, arrow keys nudge; in a bitmap‑only font it also edits the advance width), **magnifier** (⌥‑click to zoom out), **pencil** (mousedown *toggles* the bit under the cursor and the rest of the drag then sets or clears to match — a genuinely good interaction), **line**, **four‑arrow** (move the whole bitmap), **hand**. The right‑click menu adds open/filled rectangle, open/filled ellipse, rotate 90° CCW/CW/180°, flip H/V, and Set Width (bitmap‑only fonts). Anti‑aliased strikes (2/4/8 bpp) add a **Shades palette**; with the pencil, ⌥ turns the cursor into an eyedropper; the colour under the cursor is outlined in white, the selected one in green.

**FontLab's answer is a separate product, BitFonter 3** ($149, Windows; the Mac build runs under Wine on macOS ≤ 12 — effectively abandonware). It has dedicated font and glyph windows; tools Pencil, Eraser, Brush, Text, Line, Rectangle, Ellipse, Fill, Rotate; marquee / freeform / magic‑wand selection; colour depths from 1‑bit to 24‑bit RGB with alpha; image filters (Gaussian blur, sharpen, soften, brightness/contrast). Formats: **BDF, FON, FNT, PCF, PDB (PalmOS), AFP, SFP (HP PCL), and SBIT** (TrueType‑flavoured OpenType with embedded bitmaps), plus its own **Photofont (.phf)** which round‑trips through PNG files in Photoshop. It autotraces bitmaps to outlines (exported as VFB for FontLab Studio) and can generate **"Outline Pixelfont"** output where each pixel becomes a scalable square.

**Glyphs' approach is quietly the best integrated one**: a **Pixel tool (`X`, or ⇧B if the Pencil is selected)**. It requires a **grid spacing of ≥ 2** (Font Info → Other). It draws instances of a glyph literally named `pixel`; if the font lacks it, Glyphs offers to create it as a square the size of the grid spacing. Click adds a pixel; click again removes it; drag paints; starting a drag *on* an existing pixel erases while dragging. The `pixel` glyph can contain **any arbitrary shape** — an x‑mark for a stitch effect, a rounded square, a dot for an LED face — and you temporarily reduce the grid spacing to draw sub‑pixel detail inside it, then raise it again. Because every pixel is a component, one edit to `pixel` restyles the whole font. (Glyphs 4's changelog even contains "Fix performance for fonts with a lot components (e.g. pixel fonts)", confirming this is the sanctioned workflow.)

**Dedicated pixel tools.** *BitFontMaker2* (pentacom.jp) — browser, a 16×16 grid, copy/paste between characters, a public gallery, direct TTF export; extremely low friction, no strikes, no kerning. *PixelForge* (pixel-forge.com, itch.io) — desktop, 1‑bit only, single size, direct grid editing, **real‑time text preview**, full Unicode including unassigned ranges, multiple documents, TTF export **and TTF import with automatic pixel‑size detection**, and a **plain‑text project format that is version‑control friendly**. *YAL's Pixel Font Converter* — image‑sheet → TTF. *Kreative BitsNPicas* — bitmap font editor exporting TTF/OTF/BDF/etc. (unverified in detail). *FontStruct* — a brick/module system rather than a pixel grid, useful precedent for modular design. *BMFont/AngelCode* and friends target game atlases, not fonts.

**Format reality check.** BDF carries `FONT` (XLFD), `SIZE`, `FONTBOUNDINGBOX`, `CHARS`, and per‑glyph `STARTCHAR`/`ENCODING`/`SWIDTH`/`DWIDTH`/`BBX`/`BITMAP`, plus properties `PIXEL_SIZE`, `POINT_SIZE`, `RESOLUTION_X/Y`, `AVERAGE_WIDTH`, `CAP_HEIGHT`, `X_HEIGHT`. OpenType embeds monochrome strikes in **EBDT/EBLC** (with `EBSC` for scaled strikes) and colour bitmaps in **CBDT/CBLC** (Google) or **sbix** (Apple). Glyphs supports sbix through **iColor layers** named by pixel size (`iColor 512`, `iColor 24`), referencing external PNG/JPEG/TIFF files that must share an aspect ratio, with tiny paths on the master layer providing the bounding box some renderers (Chrome, Photoshop) use as a mask.

The uncomfortable truth: **monochrome embedded strikes are effectively dead on modern desktop and web stacks** (partly unverified, but browsers and macOS/Windows text stacks overwhelmingly render outlines). So the strike editor's real value is (a) authoring BDF/PCF for terminals, embedded devices and retro targets, (b) authoring *pixel‑outline* fonts that render crisply everywhere, and (c) proofing what the outline font looks like when rasterised.

### Requirements for our integrated bitmap workflow

**Data model.** A glyph has: N outline layers (per master) **and** a set of **strikes**, each strike keyed by `(ppem, bit depth)` and optionally by master. A strike stores a bitmap plus its own metrics (`bbx` offset, advance/`dwidth`, and vertical origin for CJK). Strikes are first‑class layers in the Layers panel, not a side table.

**Strike editor (its own tab kind, plus an inline mode in Edit View):**
- Tools: **Pencil** (toggle‑on‑press then paint‑to‑match, exactly FontForge's behaviour), **Line**, **Rect/Ellipse** (outline & filled), **Fill**, **Eraser**, **Marquee/Lasso select** with move/copy/nudge, **Move bitmap**, **Eyedropper** (for grey/colour strikes), **Hand**, **Zoom**. Aseprite conventions: right‑button erases, `[`/`]` change brush size, `Alt` picks colour.
- Overlays: the outline of the same glyph drawn on top at 30 % opacity, the em/baseline/x‑height/cap grid, the advance‑width column, and **onion‑skin of the previous/next glyph in the current string** for spacing.
- HUD: pointer position in pixels relative to the glyph origin, drag delta, current strike ppem, and the derived font‑unit size of one pixel (`UPM ÷ ppem`).
- **Rasterise from outline** into the strike, per glyph or batch, with threshold/gamma controls and a "keep manual edits" diff view showing which pixels the designer changed away from the automatic result.
- **Multi‑strike editing**: a strip of all ppem sizes for the current glyph; edit one, see the others; "copy strike to…" and "scale strike to…".
- **Waterfall preview** of the strike set at their native sizes, plus a 1:1 pixel preview of a sample string.

**Pixel‑to‑outline conversion** (the feature that makes this a *font* editor rather than a sprite editor):
- Trace the bitmap as square‑pixel contours, **merging runs into maximal rectangles** and then merging rectangles into a single polygon per region (greedy rectangle decomposition + polygon union), then **remove collinear points**. This is the difference between 4 points and 400.
- Correct winding per contour (outer counter‑clockwise, counters clockwise), and emit an option to *keep* overlaps (fewer points, needs remove‑overlap at export) or resolve them.
- Options: **corner treatment** — sharp / chamfer (45° cut of N units) / round (radius R) / "scanline" (round only outer corners) — plus **pixel inset/outset** for optical weight, and **gap closing** for diagonal‑touching pixels (the classic 8‑connected vs 4‑connected choice, exposed as "join diagonals: none / bevel / full").
- **Grid alignment**: pick UPM and pixel grid together. Offer presets where one pixel is an integer number of units — UPM 1000 with 10/20/50/100 u pixels, UPM 1024 with 64/128 u, UPM 2048 with 128/256 u. Enforce the grid spacing (Glyphs requires ≥ 2) and warn on non‑integer ratios.
- **Pixel‑as‑component mode** (Glyphs' model): instead of tracing, emit one component per pixel referencing a `pixel` glyph. Then a single edit to `pixel` restyles the entire font — squares → rounded squares → dots → x‑marks → LED segments. Make `pixel` a **variable component** with axes (roundness, inset, dot size) so a *variable* pixel font is one axis away. This is our headline pixel feature and nobody else has it in this form.
- Round‑trip: **import a TTF/OTF and auto‑detect that it is a pixel font** (PixelForge does this): find the greatest common divisor of all coordinates, propose a pixel size, and reconstruct the strikes.

**Formats:** import/export **BDF** (with a full properties editor — XLFD name builder, `PIXEL_SIZE`, `POINT_SIZE`, `RESOLUTION_X/Y`, `AVERAGE_WIDTH`, `CAP_HEIGHT`, `X_HEIGHT`, `FONT_ASCENT/DESCENT`), **PCF** read, **FON/FNT** read (unverified priority), embed **EBDT/EBLC** strikes into TTF, **sbix** and **CBDT/CBLC** for colour, and an image‑sheet importer (PNG grid + a mapping string, as YAL's converter does). Also export a plain **PNG sheet + JSON metrics** for game engines.

---

## 10. Preview & proofing

**Glyphs Preview Area** (eye icon bottom‑left of Edit View, draggable separator) and detachable **Preview Panel** (Window → Preview Panel): renders the full text of the current tab on one line with the current glyph centred; drag horizontally to reposition; double‑click a glyph to open it; right‑click → "Always Center Active Glyph". An instance pop‑up selects which instance to preview, with **Show All Instances** (one glyph down all instances) and per‑instance eye toggles; a dash mirrors the current master. Three controls that matter: **Invert** (white‑on‑black), **Flip** (upside‑down, the classic spacing test), and a **Blur slider** (legibility check). Text Waterfall and Glyph Waterfall modes let you set the number of steps and the sizes. **Sample Strings** live in Settings and can be organised into groups and inserted into Edit View. ⌘E opens export for real‑world testing; Test Install bypasses font caches.

**FontLab**: a **Preview panel** (⌥⌘P) plus a Preview mode in the Glyph window, `` ` `` for quick preview (hold), Space for pan‑and‑preview, ⇧Space "True Fill", **Waterfall Kerning**, and the **Pairs and Phrases panel** as the proof‑string library (↑/↓ steps through phrases while spacing).

**FontGoggles** sets the bar for shaping QA: the Features panel shows the **union of all GSUB and GPOS features across every open font**; clicking a tag cycles **grey (default) → green (on) → red (off)**, with ⌥ reversing the cycle; right‑click a GSUB tag to specify an alternate index (for `aalt` etc.); a Script button overrides HarfBuzz's automatic script detection and a Language button selects language‑specific lookups; a Variations panel shows sliders for the union of all axes across the open fonts.

**Fontra** shows how to do this *inside* a source editor without compiling a font: it runs **HarfBuzzJS (HarfBuzz compiled to WebAssembly)** and feeds it a **"shaper font" — a minimal font containing only GSUB/GPOS/GDEF and no outlines** — built on the fly by a Rust library (`build-shaper-font`) also deployed as WASM, which returns the compiled tables plus insertion markers describing where emulated features apply. Kerning, mark attachment and cursive attachment are emulated from live source data rather than compiled, so they stay interactive. The Text Entry panel exposes: a master "Apply text shaping and features" toggle, automatic RTL/LTR detection with manual overrides for direction, script and language, feature toggles grouped by kind, and an **"Input characters and output glyphs"** inspector showing the Unicode→glyph mapping, glyph metrics and cluster indices.

### Recommendation

- **Adopt the Fontra architecture.** Compile a **shaper font** (GSUB/GPOS/GDEF only, from our feature IR + kerning + anchors) in Rust on a background thread, incrementally, on every edit; shape with **HarfBuzz via `harfrust`/`rustybuzz`**; draw outlines from the live editing model, not from the compiled font. This gives correct complex‑script shaping in the editing canvas itself — which Glyphs does not have (it only previews *manual substitution* code in Edit View, never manual positioning code).
- **One proofing surface, three modes:** Text (a live‑shaped paragraph), **Waterfall** (configurable size ladder), **Grid/Glyph waterfall** (one glyph across all instances, or all glyphs at one size). Plus invert, flip vertically, blur, and a **fake‑rasterisation mode** (grayscale/subpixel at a chosen ppem) for hinting and pixel work.
- **Feature toggles with FontGoggles' tri‑state**, script/language overrides, an alternate‑index picker, axis sliders, and a **cluster inspector** showing input codepoints → output glyph IDs → x/y advances and offsets. Add a "why" affordance: click an output glyph and list the lookups that fired.
- **Compare masters / compare instances** side by side or overlaid with difference shading; **compare against a reference font file** (a previous release, or a competitor).
- **Sample‑string library** in the project file, grouped, with per‑script defaults, and a "proof sheet" export to PDF/HTML (Glyphs users currently reach for the third‑party ProofingTool plugin for this).
- Dark mode throughout, with a *separate* canvas theme setting — Glyphs lets users configure corner‑node colour, smooth‑node colour, zone colour, foreground/background/other‑layer stroke colours, canvas colour and the two kerning indicator colours **independently for light and dark appearance**. Match that.

---

## 11. Cross‑cutting UX

**Undo/redo.** Glyphs' undo is **per glyph** — every glyph has its own history. The consequence, stated in the handbook, is that some global actions (notably editing global guides, which belong to a master rather than a glyph) **cannot be undone at all**. That is a real defect to avoid. Recommendation: a **single document‑level undo stack with per‑entry scope metadata**, so the UI can display "Undo: move 12 nodes in `a` (Regular)" and can offer a per‑glyph filtered view, while global operations remain undoable. FontLab keeps a **History panel** plus "Rewind Record / Rewind / Fast Forward" for replaying edit sequences — a nice extra, low priority.

**Autosave & versioning.** Glyphs exposes a "Use Versions" preference (macOS document versioning) and warns that it modifies files on open/close, which fights git. Our answer: a **text‑first, diff‑friendly project format** (a directory of TOML/JSON per glyph, like UFO or the `.glyphspackage` model — note Glyphs 4 already splits `font.note` into its own file inside packages and reloads it when changed externally), plus **crash‑recovery snapshots in a sidecar directory** that are never written into the project. Add a `lastChange` suppression toggle (Glyphs has `Write lastChange` for exactly this reason: it otherwise churns every glyph on every save and poisons diffs).

**Multiple fonts.** Required: many documents open at once, cross‑document copy/paste with component relinking, family grouping, and a **Compare Fonts / Compare Family** view. Glyphs' **Paste Special (⌘⌥V)** is the model to copy exactly: three paste modes — *Glyphs With Same Name* (overwrite matching names, add missing), *Selected Glyphs* (overwrite the selection, keep target names), *As Components* (paste as components into the selected glyphs, all masters optional, preserving existing content) — crossed with a partial‑content picker: *All Data*, *Content of Active Layer*, *Kerning Groups*, *Metric Keys*, etc. Plain ⌘C/⌘V between files relinks components and shows a "no base glyph" triangle placeholder when a referenced glyph is missing; name collisions get `.001` suffixes.

**Drag and drop.** Image onto a glyph cell or onto the canvas places a background image (relative path stored, broken‑link indicator when it moves); SVG/AI/PDF onto the canvas imports paths; a `.ttf`/`.otf`/`.ufo`/`.designspace`/`.glyphs` onto the app opens it; a bitmap sheet onto the strike editor imports pixels; glyph cells drag between two open documents.

**Scripting.** Glyphs: Python, with `# MenuTitle:` and `__doc__` headers, a Scripts folder (⌘⇧Y to open, ⌘⌥⇧Y to reload), ⌘⌥R to re‑run the last script, a Macro/Scripting window, a Plugin Manager with Plugins / Scripts / Modules tabs and one‑click install, and five plugin types: **Reporter** (`.glyphsReporter`, draws in Edit View, appears under View → Show…), **Filter** (`.glyphsFilter`, appears under Filter), **Palette** (`.glyphsPalette`, adds an Inspector section), **File Format** (`.glyphsFileFormat`), **Tool** (`.glyphsTool`, adds a toolbar tool), **General**. FontLab: Python 3 with a Scripting panel and a Scripts menu. FontCreator: no scripting language, only the Glyph Transformer.

**Our scripting story — recommendation:** make **TypeScript/JavaScript the first‑class scripting language**, with Python as a supported second citizen.

- Embed a JS engine (QuickJS or `rusty_v8`/Deno core) in the Rust host, or — simpler and better for UI plugins — run plugin code in a **dedicated hidden webview** communicating with the core over the same typed RPC the main UI uses. Either way, **the plugin API is the same protocol the app's own UI speaks**, which guarantees it stays complete.
- Ship **generated `.d.ts` types** from the Rust API definitions, so plugin authors get autocomplete and type checking in VS Code with zero setup. This is a decisive advantage over Glyphs' loosely‑typed Objective‑C bridge.
- **Python via an out‑of‑process interpreter** speaking the same JSON‑RPC — the user's own `python3`, not a bundled one. This lets the enormous existing ecosystem (fontTools, fontMake, ufoLib2, and ported Glyphs scripts) work without us embedding CPython or owning its packaging.
- Plugin types mirroring Glyphs': **reporter** (canvas overlay), **filter** (layer transform, with a preview dialog contract), **inspector panel**, **tool**, **file format**, **exporter**, and **command**. A built‑in **plugin manager** with a curated index, one‑click install, no restart required for scripts.
- A **macro/console panel** with output capture, and every UI action recordable to a script (the "record what I just did" affordance none of the incumbents have).

**Theming, localisation, accessibility, high‑DPI.**
- Theming: full light/dark with *separate canvas palettes* per appearance, as above; a small number of built‑in themes plus a user‑editable theme file.
- Localisation: Glyphs ships a localised UI with a "Disable Localization" escape hatch (needed because designers read English docs). Do the same: localise fully, but keep an English‑UI toggle, and **never localise glyph names, feature tags, axis tags or file paths**. Prioritise en / de / zh‑Hans / zh‑Hant / ja / ko / es / fr given the CJK ambition.
- Accessibility: full keyboard operability including the canvas (arrow‑key node selection with Tab/⇧Tab traversal, which Glyphs already has); configurable node/handle sizes (Glyphs has a Handle Size setting explicitly for hit‑target size); respect OS reduce‑motion and increased‑contrast; screen‑reader labels on every panel control; **never encode information in colour alone** — every badge and every compatibility state needs a shape or glyph as well as a hue.
- High‑DPI: render the canvas at `devicePixelRatio`, keep all hit‑testing in **CSS/logical pixels** with configurable hit‑to (3 px) and snap‑to (5 px) distances like FontLab, and coalesce pointer events into `requestAnimationFrame` with a retained scene rather than redrawing per event. For the glyph grid, cache rasterised cells in a texture atlas keyed by (glyph, master, cell size, DPR).

---

## 12. Prioritised feature checklist

*Legend:* **[CJK]** = CJK‑specific or CJK‑critical.

### Tier 1 — Must have for v1

**Shell & document**
1. One window per document; tab strip with pinned Overview tab (⌘⌥1) + Edit tabs (⌘⌥2–9, ⌃⇥ cycles)
2. Tear‑off tabs and panels into separate OS windows
3. Right Inspector dock as a contextual scrolling section stack; left context sidebar
4. Collapsible bottom preview strip with draggable divider
5. Command palette (⌘K) over commands, filters, scripts, glyph names
6. Saved workspaces, restored per document
7. Text‑first project format (per‑glyph files), git‑friendly; crash‑recovery snapshots in a sidecar
8. Document‑level undo stack with per‑entry scope labels; one entry per gesture
9. Cross‑platform parity: macOS, Windows, Linux, identical keymaps modulo ⌘/Ctrl

**Font view**
10. Virtualised GPU grid, cell‑size slider, grid/list toggle, sortable & rearrangeable list columns
11. Cell badges: metrics out of sync, annotation, custom glyph info, incompatible masters, non‑exporting, locked
12. 12 named colour labels + separate layer colour; free‑text tags with autocomplete
13. Search: name / Unicode / note, case‑sensitivity toggle, regex
14. Sidebar: Categories & subcategories, Scripts/Languages, Filters, Lists, with present/total badges and "generate missing glyphs"
15. Smart filters with the full predicate set, saved in the project, editable as text DSL *and* visual builder
16. Glyph database (name ↔ Unicode ↔ category ↔ production name ↔ anchors ↔ composition) as a shippable data file with user overlay; "Update Glyph Info"
17. Add Glyphs by name / character / codepoint / range / recipe; batch rename with find‑and‑replace
18. **[CJK]** CJK cells rendered from a fallback system face when empty; radical / stroke‑count / Unicode‑block sidebar sections
19. **[CJK]** Vertical metrics in the inspector (TSB/BSB, vertical origin, vertical advance)

**Edit view**
20. Tools: Select, Lasso, Text, Pen, Pencil, Primitives (rect/ellipse), Knife, Eraser, Rotate, Scale, Measure, Zoom, Hand, Annotation, **Pixel**
21. Tap‑to‑switch / hold‑to‑borrow tool activation
22. Smooth vs corner nodes with double‑click / Return toggle; ⌥ variant preserving the outline
23. G2 "super‑smooth" nodes + one‑shot Harmonize
24. Full modifier grammar: ⇧ constrain/×10, ⌘ ×100, ⌥ alternate behaviour, ⌃ suspend snap
25. Nudge (proportional handle adjustment) while dragging nodes
26. ⇧‑click a segment inserts a node at the nearest extremum/inflection; Add Extremes / Force Extremes
27. Tidy up Paths, Correct Path Direction (+ all‑masters variants), Reverse Contours, Open Corner / Reconnect Nodes
28. Info box with editable X/Y/W/H, transformation origin, proportional lock, Tab traversal, ↑/↓ increments
29. Cursor‑following HUD: delta, angle, length, stem width
30. Local and global guides, angle lock, naming, LSB/RSB anchoring, measurement guides
31. Snapping to nodes, handles, component interiors (with ghost nodes), metrics, zones, guides, grid, background — each independently toggleable, with configurable hit/snap distances
32. Background layer: show, edit (⌘B), selection→background, swap, clear, per‑master
33. Components with automatic alignment via anchors; decompose; lock component
34. Anchors with `_`‑prefixed mark anchors and a mark‑cloud preview
35. Always‑on outline audit with inline markers, hover detail, Fix / Fix All, per‑test toggles
36. Layers panel: masters, backups, background, per‑glyph layer add/duplicate/rename

**Spacing & kerning**
37. Metrics keys with the full syntax (`=n`, `=n±k`, `=|n`, `=50`, `==n` local, `=n@y`, `=+k` on auto‑aligned)
38. Out‑of‑sync indicators + Update Metrics (current / all masters)
39. Spacing shortcuts with LSB / RSB / both, ×1 / ×10 / ×100 / true‑1
40. Kerning modes: off / on / **locked** (kern without risk of respacing)
41. Kerning groups per side; class‑to‑class by default; modifier‑held class suppression for exceptions
42. Kerning colour coding in canvas (negative vs positive) and group‑member ghosting
43. Kerning table: sortable, filterable, group rename with scope prompt, Clean Up, Compress, copy/paste between masters
44. Spacing tab with editable metrics matrix, beam measurement, cut‑off display, saved proof strings
45. **[CJK]** Vertical kerning and vertical metrics editing

**Features**
46. FEA parse/serialise round‑trip into a typed lookup IR
47. Prefix / Classes / Features tree with per‑entry auto vs manual
48. Automatic generation from glyph‑name suffixes (liga, dlig, smcp, c2sc, ss01–ss20, onum/lnum/tnum/pnum, frac, ordn, sups/sinf/numr/dnom, locl, ccmp, aalt)
49. Implicit kern/mark/mkmk/curs generated from kerning and anchors
50. Inline error diagnostics with jump‑to‑source; autocomplete; ⌥‑click class preview
51. **[CJK]** `vert`/`vrt2` generation and vertical alternates handling

**Font Info / variable / export**
52. Scoped info: Family → Font → Master → Export with inheritance and override indicators
53. Per‑master metrics, alignment zones, stems; UPM, grid spacing & subdivision, keyboard increments
54. Axes, masters at coordinates, static instances, ≥1 variable‑font config with designspace subsetting
55. Compatibility view: shape‑order colours & indices, cross‑master connectors, green/yellow/red segments, drag‑to‑swap repair
56. Intermediate (brace) and Alternate (bracket) layers with a proper condition editor
57. Live axis sliders scrubbing the drawing in Edit View
58. Export profiles stored in the project; instance × profile matrix; preflight report with click‑to‑jump
59. Formats: OTF (CFF), TTF, WOFF2, WOFF, variable TTF, UFO + designspace
60. Remove overlap, extrema insertion, coordinate rounding, production names, decompose, subroutinize
61. Autohinting (ttfautohint for TT, PS autohint from stems/zones)
62. Test install on all three platforms
63. CLI sharing the same profiles for CI

**Bitmap**
64. Strikes as first‑class layers keyed by (ppem, depth), with per‑strike metrics
65. Strike editor: pencil (toggle‑then‑match), line, rect, ellipse, fill, eraser, marquee select/move/copy, move bitmap, eyedropper, zoom, hand
66. Outline overlay, pixel grid, advance column, pointer/delta HUD, units‑per‑pixel readout
67. Rasterise from outline (per glyph and batch) with threshold control
68. Pixel‑to‑outline: rectangle merging, collinear removal, correct winding, corner mode (sharp / chamfer / round), diagonal join mode
69. Pixel‑as‑component mode driven by an editable `pixel` glyph
70. Grid presets guaranteeing integer units per pixel (1000/1024/2048 UPM families)
71. BDF import/export with a full properties editor; PNG sheet import/export

**Preview**
72. Live HarfBuzz shaping in the editing canvas via an incrementally compiled GSUB/GPOS‑only shaper font
73. Preview strip + detachable preview window; instance picker; invert; flip; blur
74. Waterfall (text and single‑glyph) with configurable steps
75. Feature toggles (tri‑state), script/language override, axis sliders, cluster inspector
76. Sample‑string library in the project, grouped, script‑aware

**Cross‑cutting**
77. Paste Special: same‑name / selected / as‑components × all‑data / layer / kerning groups / metric keys
78. Cross‑document copy with component relinking and missing‑base placeholders
79. TypeScript scripting API with generated `.d.ts`; macro console; scripts folder + reload
80. Full light/dark theming with separate canvas palettes; localisation with an English‑UI toggle
81. High‑DPI canvas, logical‑pixel hit testing, configurable handle size

### Tier 2 — Should have

82. Smart Components / variable components with per‑instance axis sliders and smart handles **[CJK critical]**
83. Corner, cap, segment and brush components (`_corner.*`, `_cap.*`, `_segment.*`, `_brush.*`)
84. Live smart corners with numeric radius, positive = round / negative = ink trap
85. Node‑linked guides (Power Guides) with a magnet tool and automatic stem detection
86. Servant nodes (X/Y interpolated positions)
87. Non‑destructive strokes with pen points (width, height, angle, position, progression) and caps/joins
88. Masking / boolean shape ops as non‑destructive layer effects
89. Path focus and path/glyph locking
90. Measurement tool with rulers, intersection distances, italic‑angle‑aware deltas, and drop‑a‑guide‑mid‑drag
91. Annotation tool (text, arrow, circle, plus, minus) with per‑layer storage
92. Background images with relative paths, transform, lock, and broken‑link indicators
93. Autotrace from raster images
94. Auto‑spacing (optical metrics with a global density factor) and auto‑kerning
95. Pairs‑and‑phrases proof library with ↑/↓ stepping during spacing
96. Compare Fonts / Compare Family diff view; compare against a reference binary
97. Virtual masters
98. Manual PostScript hint editing (stems, ghosts, hint replacement)
99. Visual OpenType lookup editor for GSUB 1/2/3/4/6 and GPOS 1/2/4/6, round‑tripping to FEA
100. Feature‑code tokens (numbers, glyph properties, predicates) and `#ifdef VARIABLE` conditional blocks
101. Multi‑strike waterfall and cross‑strike copy/scale
102. EBDT/EBLC strike embedding in TTF; sbix and CBDT/CBLC colour bitmaps
103. TTF/OTF import with pixel‑font auto‑detection and strike reconstruction
104. Colour fonts: COLR/CPAL (v0 at minimum), layered colour layers, palette editor
105. Python scripting via an out‑of‑process interpreter; plugin manager with a curated index
106. Reporter plugins drawing on the canvas; filter plugins with a preview contract
107. Quick Help on hover (F1) and a context help panel
108. Importable keymap presets ("Glyphs", "FontLab", "RoboFont") and a shortcut editor with conflict detection
109. Proof sheet export to PDF/HTML
110. **[CJK]** CID mapping, CID‑ordered sort, and CID‑keyed export
111. **[CJK]** Vertical writing mode in Edit View and in all proofing surfaces, with `VORG` and vertical origin editing
112. **[CJK]** Component‑based CJK construction helpers: radical library, ideographic description sequence input, per‑component transform grid

### Tier 3 — Later

113. Higher‑order interpolation (curved node paths, rotation centres, timing)
114. Visual TrueType instruction editing (Snap / Stem / Shift / Interpolate / Delta) with three‑outline display and rendering‑intent switching
115. `gasp`, CVT and control‑value program editing
116. Matchmaker‑style semi‑automatic master matching (select two node groups → insert/remove nodes across masters)
117. Sketchboard / free canvas for arranging drawings outside glyph slots
118. Tunni lines and curve‑tension direct manipulation
119. Edit‑action recording and replay (Rewind Record)
120. Multiplayer / shared editing session (Figma model); comment threads on glyphs
121. COLRv1 with gradients and blend modes; SVG‑in‑OpenType
122. avar2 / axis mappings beyond a simple mapping table; STAT authoring UI
123. Photofont‑style bitmap round‑trip through an external image editor
124. Greyscale and full‑colour bitmap strikes with a shades palette and eyedropper
125. Modular/brick design mode (FontStruct‑style) built on the pixel‑component engine
126. **[CJK]** Automatic component derivation from an existing CJK font (decomposition into reusable radicals)
127. **[CJK]** Kanji/Hanzi coverage sets (JIS levels, GB, Big5, Adobe‑Japan1‑x, Adobe‑GB1, Adobe‑CNS1, Adobe‑Korea1/KR) as one‑click character‑set targets with progress tracking
128. **[CJK]** Han‑specific proofing: 字面 (face size) grid overlay, centre‑of‑gravity guides, stroke‑count QA reports

---

### Three closing judgements

**The Rust/TS stack is the right bet, and the pieces exist.** `kurbo` for curves, `norad` for UFO, `fontations` (`read-fonts`/`write-fonts`/`skrifa`) for reading and building tables, `fontc` as a reference compiler, `harfrust`/`rustybuzz` for shaping, `ttf-parser`/`swash` for rasterisation. The one place to be careful is autohinting: ttfautohint is a C library and `psautohint` is Python — plan on FFI or a subprocess rather than a Rust rewrite for v1. (Maturity assessments here are my own reading and partly unverified as of 2026.)

**Two things would make us obviously better than the incumbents rather than merely equivalent:** (a) **live HarfBuzz shaping of the source in the editing canvas** via an incrementally compiled shaper font — Glyphs cannot preview manual GPOS in Edit View at all; (b) **a text‑based, git‑diffable project format with a CLI that shares the GUI's export profiles**, which turns font production into normal software engineering.

**The pixel story should be built on variable components, not on a bolted‑on bitmap mode.** A `pixel` glyph with axes (roundness, inset, dot size) placed as thousands of variable components gives, from one editor primitive: a pixel drawing tool, instant restyling of an entire pixel typeface, a variable pixel font, and a clean pixel‑to‑outline path — while the strike editor handles BDF and embedded‑bitmap output for the terminal and embedded markets. No competitor has both halves in one application.


---

# Errata / verification pass 1

Follow‑up: a second verification pass against the **Glyphs 3.0.4 handbook** (the last Glyphs‑3‑specific one, now only available via the Wayback Machine) came back after I delivered the document. It confirms almost everything above, but produces a short errata/addenda list worth folding in. Three further verification passes (FontLab, FontCreator + bitmap tools, Fontra/RoboFont/Rust stack) are still running; I'll relay anything material when they land.

## Corrections to the delivered document

- **§2 font‑view sidebar.** In **Glyphs 3** the sidebar has exactly three sections — Categories, Languages, Filters — with List Filters and Folders living *inside* Filters. The separate "Lists" section I listed is the **Glyphs 4** arrangement.
- **§3 tool shortcuts.** `I` is the **TrueType Instructor** tool in Glyphs 3; Glyphs 4 reassigns `I` to the new Higher‑Order Interpolation tool. Both facts are correct, but they belong to different versions — worth knowing when we pick our own defaults.
- **§4/§12 vertical kerning.** There is **no "Kern Vertically" command** in Glyphs (the brief assumed one). Vertical kerning is a consequence of setting the writing direction to top‑to‑bottom, which swaps the glyph to top/bottom kerning groups and the vertical Info box (T/B/O/H/K/G). My checklist item is right in substance; the mechanism is a mode, not a command.
- **§4 kerning group class names are not legacy.** A **left** kerning group compiles to `@MMK_R_<name>` and a **right** group to `@MMK_L_<name>` — the letter refers to the *other* glyph in the pair. Current behaviour, not a Glyphs‑2 holdover. Our compiler must reproduce this if we want FEA round‑trip parity.
- **§5 implicit features.** The full implicit set is `kern`, **`dist`**, `mark`, **`abvm`**, **`blwm`**, `mkmk`, `curs` — I listed four of seven. The `# Automatic Code` placeholder line is how a user interleaves manual code with generated code; we need an equivalent.
- **§7 variable preview.** Glyphs 3's built‑in preview offers an instance pop‑up only; **axis sliders were a third‑party plug‑in** (Mark Frömberg's Variable Font Preview) until Glyphs 4 added the Axes sidebar. That strengthens the case for making live axis scrubbing a Tier‑1 item for us rather than Tier 2.

## Additions worth putting into the spec

- **Smart‑filter rules I under‑listed:** Has Metrics Keys, Metrics Keys Out of Sync, Has Kerning Groups, Color Label, Layer Color Label, **Is Hangul Key**, and a **Custom** glyph‑predicate expression — plus **nested Any/All/None groups** (Option‑click the "…" button). Our filter DSL should therefore support boolean nesting from the start, not just a flat AND list.
- **CJK custom parameters that should become first‑class settings in our app** (they are the concrete CJK design‑grid story I gestured at in §12): `CJK Guide` (virtual‑body inset %, script‑scopable as `kana:5`), `CJK Grid` / `CJK Grid Horizontal` / `CJK Grid Vertical` (dotted design grid; 8 is the recommended value for Hangul), `Design Grid` (`script:h,v`), `Hangul Composition Groups` (drives the "Is Hangul Key" filter), and `Default Layer Width` which defaults to **1000 units for CJK glyphs** vs 600 otherwise.
- **CJK vertical metrics trap:** for a font intended for vertical layout, `typoAscender` must be set to the **top of the ideographic em‑box** (e.g. 880 for a 1000×1000 box sitting 120 units below the Latin baseline) or vertical layout breaks. Our Font Info validator should check this.
- **Hangul jamo naming:** `-ko` suffix with position abbreviations `Cho` (initial), `Jung` (medial), `Jong` (final) — e.g. `mieumCho-ko`. Our glyph database needs these.
- **`vrt2` is auto‑generated from the `.vert` suffix**; plain `vert` is not in the documented auto‑generation table. Adjust checklist item 51 accordingly.
- **Feature tokens, precisely:** `$name` (interpolating Number Values), `${name*2}` (arithmetic), `${glyph:property}` (metrics and anchor coordinates, e.g. `${b:anchors.top.x}`), and `$[predicate]` (glyph lists, with `in class(...)`, `AND/OR/NOT`). Option‑clicking a token previews its expansion. This is a strong argument for our feature IR carrying *expressions*, not just literal glyph names.
- **Supporting evidence for the Rust bet in my closing note:** Glyphs 3.3 shipped an **"Export with fontc"** option — Google's Rust font compiler, delivered as a file‑format plug‑in. The incumbent is already routing production export through the stack we plan to build on.


---

# Errata / verification pass 2

Second verification pass in — this one on **FontLab**. It contains one outright error in my delivered document and several corrections worth acting on. (The FontCreator/bitmap and Fontra/RoboFont/Rust passes are still running.)

## Errors in the delivered document

- **§1 — my "command palette" claim is wrong.** I wrote that a command palette is "the single highest‑leverage modern affordance none of the three incumbents have." FontLab has one: `Tools > Commands & Shortcuts` (**⇧⌘P**) doubles as a command palette. The recommendation stands, but it is table stakes, not a differentiator. Related: FontLab's single‑key shortcuts (V, L, …) **cannot** be rebound, and shortcut sets import/export as JSON — we should allow rebinding everything and ship the same JSON portability.
- **§3 — FontLab's component model is two mechanisms, not one.** I described "elements" as its component model. FontLab 8 has **both**, deliberately: **Components** (point at a designated source glyph layer, know its metrics and anchors, behave like UFO/TTF/VFB components) and **Element References** (peer links with no source-of-truth, no knowledge of metrics or anchors, granularity of *any sub‑shape* — a stem, a serif — with per‑instance fill colour, stroke colour and thickness; the docs compare them to CFF subroutines). Earlier FontLab versions faked components with element references and they split them apart precisely because the two jobs are different. Our model should make the same split rather than picking one.
- **§9 — FontLab is not bitmap‑blind.** I wrote that its bitmap answer is the separate BitFonter product. In fact FontLab 8 **opens BDF** and converts it to a pixel font where *"each pixel is represented by a small vector square"* (a fused contour of squares), and it opens BitFonter **Photofont (PHF + PNG/SVG)**. Import only — BDF is in no export list. It also documents the Grid as being "particularly for modular or pixel‑style fonts", with a hard **round‑to‑grid** snapping mode. So the pixel‑outline half of our §9 plan has a partial precedent; the strike‑editing half still does not.

## Corrections and additions worth folding in

- **§6 Font Info.** PostScript Hinting, TrueType Hinting, PANOSE and the OS/2 sub/superscript values are **inside** an "Other Values" page, not separate pages. More usefully, FontLab has a **Font Info → Overview** page: every property as a spreadsheet with **one column per master**, rectangular range selection, copy/paste between masters *and fonts*, round‑tripping through Excel/Numbers/Sheets — and 8.4.2 **highlights rows that should be identical across masters but differ (yellow) and flags statistical outliers in red**. That is a better version of the "flat searchable view" I recommended, and the cross‑master outlier detection should go into our Tier‑1 validation list.
- **§8 export.** FontLab 8 already does the **fonts × profiles matrix** I proposed (FL7 allowed one profile at a time; FL8 allows several fonts and several target profiles in one step) — so that recommendation is validated, not novel. It also has **`postprocess(data)`** (8.3): a Python hook called after every export receiving format, profile and output paths, intended for QA and Git integration. That is close to our CLI story and worth beating rather than matching.
- **§3 snapping.** Hold **Ctrl** to suppress snapping in FontLab too (I attributed that gesture to Glyphs alone). More interesting is the **Suggest** system: red dotted lines with arrowheads proposing alignments — nodes, handles, continuation lines, perpendiculars, stem/counter centrelines, tangents, bounds, and a per‑master "suggest distance" — with a preference for whether they appear **immediately, on mouse idle, or after a delay**. That timing preference is a small thing that makes the difference between helpful and maddening; we should copy the preference, not just the feature.
- **Small ergonomics worth stealing:** numeric fields accept **calculations** (type `300+50`) and drag‑as‑slider; **⌘/Ctrl+Enter** applies‑and‑closes Preferences, Font Info and Actions; **F1 hover Quick Help** and **⇧F1 Help panel** (already in my §1); workspaces are portable **`.fl_workspace`** files; FL8 "Widgets" are floating in‑canvas property controls for the glyph, components and selection — the concrete form of the cursor HUD I recommended.
- **Auto‑spacing detail for §4:** FontLab's `;` offers three methods — **optical**, **tight** (both sidebearings 0) and **custom** (a percentage of UPM) — and its newer **K2** autokern replaces the legacy K1 (still reachable via ⇧`;`). There is also `Font > Kerning > Audit Exceptions`, which finds and fixes visual clashes in class‑based pairs. Worth having all three spacing methods rather than one.

## The most useful finding for our positioning

**FontLab's own documentation states the CJK limitation plainly:** *"CJKV fonts may be imported into FontLab but they can only be generated as TrueType fonts and will have no vertical metric sidebearings."* And `vmtx`, `VORG`, `vert`/`vrt2` are **never named anywhere in the FontLab 7 or 8 documentation** — so vertical‑layout output is undocumented at best. FontLab's answer to CJK is variable components plus Unicode Variation Selectors (`cmap` format 14, new in 8.4), not vertical typesetting.

That sharpens the competitive picture in §12: on CJK, Glyphs has the design‑side story (smart components, CJK guides/grid, top‑to‑bottom writing mode) and FontLab has the reuse story (variable components), but **neither ships a documented, complete vertical‑metrics and vertical‑feature pipeline**. Full `vmtx`/`VORG`/`vert`/`vrt2`/`vpal`/`palt` support with vertical proofing is a defensible v1 differentiator, and I would promote it from my Tier‑2 CJK items to Tier 1.


---

# Errata / verification pass 3

Third verification pass in — **FontCreator**. It corrects two factual claims in the delivered document. (The Fontra/RoboFont/Rust‑stack pass is the last one outstanding.)

## Errors in the delivered document

- **§1 — FontCreator is not MDI.** I described it as "historically Windows MDI (child windows inside a parent frame)" and called MDI "the weakest of the three models". The word MDI appears nowhere in its manual. The actual shell, per the manual's own annotated interface legend, is a **single window with Font and Glyph tabs plus docked panels**: Menu Bar, Standard / Tools / Drawing / Overview / Grid / Align‑or‑Distribute / Glyph toolbars, a **Font Overview** grid inside the Font panel, a **Font Categories** panel beside it (with Color Glyphs, Scripts and Unicode Character Blocks subtrees), and docked **Glyph, Glyph Properties, Preview, Masters and Layers, Variations, Validation, Anchors** and **Transform** panels. That is much closer to the layout I recommended than I gave it credit for — it strengthens the recommendation rather than weakening it, but my characterisation was wrong and the dismissal was unearned.
- **§1 / §3 — cross‑platform and format support arrived in v15, not v16.** **FontCreator 15** was the first macOS release, and it brought `.glyphs` support, CFF2 *loading*, Unicode 17, guideline management and **kink detection between masters**. **v16** (2026‑04‑29) added non‑destructive Mask subtraction, the redesigned Masters and Layers panel, **Path Decorations**, **Smart Components** and CFF2 *export*. I folded v15's features into v16.

## Additions worth folding in

- **§3 — FontCreator's editing model is a mode toggle, not a tool palette.** **Contour mode vs Point mode**, switched via `View → Mode`, the triangle buttons on the Drawing toolbar, or a double‑click in the canvas. The **status bar doubles as the measurement readout** (coordinates for one point, angle for one, distance between two). There appear to be **no standalone Knife, Measure or Zoom tools** *(unverified)* — instead of a knife it offers `Join and Split Contours` and a **"Split self‑intersecting contours"** command. Worth knowing that a serious editor ships without a knife tool; it suggests the knife is less load‑bearing than the Glyphs/FontLab toolbars imply.
- **§2 — a second precedent for a shipped composition database.** FontCreator's **Complete Composites** (right‑click a glyph or range) has four modes: **Auto**, **Anchor Based**, **Anchor Based Reposition**, and **Composite Data** — the last driven by a bundled **`CompositeData.xml`** covering "over three thousand glyphs". Together with Glyphs' `GlyphData.xml`, that is two of three incumbents shipping an editable composition database as a data file. My §2 recommendation to ship one with a user overlay is the consensus design, not a bet.
- **§3 / §8 — two concrete commands to copy.** **"Get Union of Contours"** on the Glyph toolbar merges intersecting composite members into a single simple glyph interactively (their answer to Ç). And on export, **"Calculate overlap flags"** sets the per‑glyph overlap flag for glyphs and composites with overlapping contours — a cheap, correct alternative to destructive overlap removal for TrueType output, and it belongs in our export‑profile option list next to "Remove overlap".
- **§5 / §10 — FontCreator's OpenType story is older and deeper than I implied.** v13 already shipped the **Anchor Manager**, an **OpenType Explorer**, **interactive OpenType proofing**, SVG color fonts, and **VOLT import/export**; v14 added variable fonts, masks, the anchor cloud, designspaces, "Edit Across Layers" and boolean contour ops. The visual‑OTL‑plus‑code combination I singled out in §5 has had three release cycles of refinement behind it, which raises the bar for our own feature editor.

One caveat on sourcing: the FontCreator manual site is a JavaScript help viewer that serves the same shell page for every topic URL, so these details come from archived captures of the v15 manual pages (2023–2025) plus the changelog and release announcements. The v15 "Using Glyph Transformations" document is a scan, so the **Glyph Transformer's script vocabulary remains unverified** — worth a direct look if we decide to copy that feature.


---

# Errata / verification pass 4

Final pass in — Fontra, RoboFont, the Rust ecosystem and adjacent-app UX. All four verification passes are now complete. This one contains the single most consequential finding of the whole exercise, plus corrections to my closing technical judgement.

## The finding that changes the v1 plan

**There is no CFF or CFF2 charstring compiler anywhere in the Rust font stack.** `write-fonts` issue #666 has been open since October 2023; `ps/cff.rs` only builds INDEXes for subsetting. **`fontc` 1.0.0** (shipped 2026‑09‑01) is **TrueType output only** — no CFF/OTF, no CFF2, no PostScript hinting, and no overlap removal in static builds. Google Fonts hints post‑compile with gftools/ttfautohint because fontc has no hinting of any kind (issue #51, untouched since 2022).

My §8 checklist lists **"OTF (CFF)" as a Tier‑1 must‑have for v1**. On the stack I recommended, that item has no implementation path. Three options, in order of preference:

1. **Write the CFF charstring compiler ourselves** and upstream it. It is a well‑specified, self‑contained problem (charstring encoding, subroutinizer, Private DICT, hint operators) and it would be a genuine contribution that the whole ecosystem is waiting on. Budget it as real work, not a weekend.
2. **Shell out to `fontmake`/`fontTools`** for OTF output in v1, keeping TTF/variable native. Pragmatic, but it means bundling Python and undermines the "single binary, CI‑friendly CLI" story I argued for.
3. **Ship TTF and variable TTF only in v1**, and be explicit about it. Defensible for a pixel‑font‑forward editor, indefensible for anyone who expects to match FontLab.

I would pick (1) with (2) as the interim bridge, and I would decide this before anything else, because it sets the scope of the whole export subsystem.

## Errors in my closing technical judgement

- **`rustybuzz` is archived** as of 2026‑07‑26 — its own README says "unmaintained… switch to HarfRust." I offered it as an alternative to `harfrust`. Use **`harfrust` 0.13.3** only: it matches HarfBuzz v14.3.1, is within 25% of HarfBuzz's speed (rustybuzz was 1.5–2× slower), shares `read-fonts` with skrifa so we don't ship two OpenType parsers, and HarfBuzz itself can be built with it as the shaper for conformance checking.
- **`ttf-parser` is maintenance‑only** — "bug fixes only… for new projects we recommend fontations." I listed it as a rasterisation option. Drop it.
- **`kurbo` has no boolean operations**, only line×segment intersection, and `CubicOffset` was removed in 0.12. Remove‑overlap is Tier‑1 in my checklist and has no kurbo implementation. The nearest fit is **`linesweeper` 0.4** (robust Bentley‑Ottmann over Bézier paths, kurbo‑native), which is what the current Runebender revival uses — but it self‑describes as early beta and its canonical repo has moved off GitHub to Radicle. Alternatives are `flo_curves`, Graphite's `bezier-rs`, or `skia-pathops` via FFI (which is what Fontra uses). This is a second unbudgeted dependency risk.
- **The hinting picture is better than I said.** I wrote "plan on FFI or a subprocess rather than a Rust rewrite." In fact skrifa's `autohint/` is a complete port of FreeType's `autofit` that **ships in Chrome as the FreeType replacement in Skia**, and since 2026‑05 it exposes `HintPlan`, `GlyphStyles`, `EdgeAction` and — critically — **`QuirksMode::Aot`, which matches ttfautohint's output** rather than FreeType's. **`tilvisan` 0.2.0** (Simon Cozens) is a ttfautohint‑compatible CLI and control‑file implementation built on that. Caveat: 66 downloads, one maintainer, five months old. Keep ttfautohint as the cross‑check, but a pure‑Rust autohinter is now plausible for v1.

Two smaller stack notes: **`glyphs-reader`** (inside fontc) is **read‑only** for the font model, so `.glyphs` round‑trip needs **`glyphslib` 0.2.7** instead; and **`norad` 0.18.4** has no designspace v5 support (no discrete axes, no `<mappings>`, no axis labels), saves glif 2.0 only, and has no lazy glif parsing — all of which constrain the document format I recommended in §11.

Two free wins I should have listed: **`interpolatable` 1.1.2** is a Rust port of fontTools' interpolatability checker, usable directly as the editor lint behind my §7 compatibility view; and **`img2bez`** is a raster→bezier autotracer that produces *font‑quality* outlines (points at extrema with H/V handles, lines kept as lines, inflections, G2‑harmonised joins, minimal point count) on kurbo + norad — that is most of my §9 pixel‑to‑outline requirement already built.

## Corrections and sharpenings to the document

- **§9 — the framing should be sharper than I made it.** There are two incompatible models and the UI follows from which you pick. *Pixel‑as‑outline* (BitFontMaker2, FontStruct, and Glyphs' Pixel tool) is really an outline editor with a snapping constraint. *Pixel‑as‑strike* (FontForge's Bitmap View, gbdfed, BitFonter) needs a **size/strike selector as permanent chrome**, because "which ppem am I editing" is always live state. My §9 quietly proposed both without saying so. We should build the pixel‑as‑outline model as the primary editor and the strike editor as a distinct mode with its own chrome — closer to Affinity's **Personas** (Draw / Pixel / Export switch tools, panels *and* menus over one document) than to a single toolbar serving both a bezier pen and a pencil. Also worth noting: **FontStruct's brick model** — the cell holds a *brick* from a palette (squares, triangles, quarter‑circles, diagonals), with composite bricks made by stacking two in a cell, exporting true outline TTF so curves stay curves. That is a third model and a good fit for a modular‑display‑face mode later.
- **§11 scripting — the Figma precedent validates my QuickJS suggestion, and explains why.** Figma originally sandboxed plugins with the Realms shim and moved to **QuickJS compiled to WebAssembly** after repeated escapes caused by object confusion between inside and outside the sandbox; QuickJS makes that impossible because the object representations are too different. The shape to copy: **plugin logic in a QuickJS sandbox with no I/O and no `invoke`**, reaching the font only through an API we define; **plugin UI in a separate WebView** with DOM but no document access; the two halves communicate only by message passing. Also copy the manifest in spirit — declared network domains **with human‑readable reasoning**, declared permissions, an `editorType` equivalent (`outline`/`bitmap`/`both`), and `menu` + `parameters` so a plugin is invocable from the command palette with typed args and no UI at all.
- **§11 undo — Fontra has the mechanism I was describing, and it is worth copying literally.** A change is a JSON object `{"p": path, "f": function-name, "a": args, "c": child-changes}`; `applyChange` walks the path, resolves the function, applies it. Undo is a rollback change generated alongside the forward change; multi‑window and multi‑user sync is broadcasting the same object; and it doubles as the scriptable mutation API. One mechanism gives undo, live sync and scripting. Combined with **Blender's operator model** (every mutation a serializable operator with typed parameters, plus F9 "Adjust Last Operation" to re‑run it with different parameters), this is the architecture to design first — the command palette, macro recording, repeat‑last, the plugin API and undo all fall out of it. Font work is overwhelmingly "almost right, try two units less," so adjust‑last‑operation removes an entire class of friction.
- **§1 — my "one window per document" recommendation has a competing precedent worth weighing.** RoboFont ships **both** Multi Window and Single Window mode as a preference, and Fontra gets multi‑window free because a view is a URL — copy the URL, get a second window on the same font. If our tabs are addressable by a route, tear‑off costs almost nothing.

## Prior art we should study rather than rediscover

**`shift-editor/shift`** (shift.graphics, GPL‑3.0) is the closest existing analogue to our stack: **Electron + React + TypeScript with Canvas 2D rendering**, over IPC to Rust crates — `shift-font` (glyphs/sources/axes object model), `shift-store` (**canonical documents in SQLite** with durable recovery), `shift-cli`. Two of its decisions are worth arguing about explicitly: SQLite as the canonical document rather than a text format (which cuts against my git‑diffable recommendation), and **stable IDs as identity with names and Unicodes as editable metadata** (which cuts against the Glyphs model my §2 leaned on, where the glyph name *is* the identity). The ID‑based model is more robust to renaming and is what Figma does for component overrides — override survival keyed on matching names breaks the moment someone renames a layer. I now think we should use stable IDs internally and treat names as metadata, while keeping name‑based addressing in the scripting API.

**Two Rust‑GUI postmortems that justify our stack choice.** Runebender died in 2021 when its developer moved to what became fontc/fontations, and because Druid — its host framework — was itself unfinished. Its 2025–26 revival by Eli Heuer ships two frontends over one core, and its **`XILEM-GAPS.md`** is the best available audit: no native menu bar in winit ("a font editor with no File menu is not a font editor anyone will use"), no global shortcut dispatch, no headless render path, and **996 lines of pure framework‑gap scaffolding versus roughly zero in the GPUI build**. **MFEK** ended with the death of its sole maintainer in January 2026; its instructive failure was pursuing modularity *before* a usable whole — the glyph editor reached beta while the UFO layer that would have made it a *font* editor never shipped — plus two full graphics‑stack rewrites that produced no user‑facing features. Both point the same way: **a webview frontend sidesteps the menu/shortcut/dialog gap that has killed native Rust font editors**, and we should ship a narrow but complete vertical slice before generalising anything.

**One Tauri constraint to design around.** Tauri v2 IPC supports raw binary both ways (`tauri::ipc::Response::new(bytes)` hands the frontend an ArrayBuffer with no JSON), but the floor is the webview's fetch path. A community benchmark puts a 10 MB payload at **~5 ms on macOS and ~200 ms on Windows** (indicative, not rigorous). The rule that follows: **IPC carries geometry, never pixels.** A glyph is a few hundred points — ship flat `Float32Array`s and render with `Path2D`/WebGL, keeping hit‑testing and handles in JS where the pointer events already live. Reserve Rust‑side rasterisation for non‑interactive surfaces (waterfalls, proof sheets, thumbnails, hinted‑at‑ppem previews). If the canvas hot path ever becomes the bottleneck, the precedent for the next step exists: `runebender-web` compiles Vello + kurbo to WASM and runs the renderer *inside* the webview with a WebGPU path, eliminating IPC from the loop entirely, while native Rust keeps file I/O, compilation and threads. Also: UFO is a *directory*, so we need `tauri-plugin-dialog`'s folder picker plus **`tauri-plugin-persisted-scope`** to keep filesystem grants across restarts.

**Suggested pins**, if useful: `norad 0.18.4` (+kurbo, rayon, ufoz), `glyphslib 0.2.7`, `kurbo 0.13.1`, `linesweeper 0.4`, `skrifa 0.46.2`, `read-fonts 0.43.3`, `write-fonts 0.52.0`, `skera 0.6.0` (the subsetter, renamed from `klippa` in February), `harfrust 0.13.3`, `fea-rs 1.0.0`, `tiny-skia 0.12` or `vello_cpu 0.2`, `fontc 1.0.0` with `default-features = false`, optionally `tilvisan 0.2.0`. Pin `write-fonts` and `skrifa` exactly — they went 0.44 → 0.47 → 0.49.1 → 0.52.0 within a single fontc release cycle. One more heads‑up: `otexplorer` and the whole `experimental_traverse` reflection API were **deleted on 2026‑08‑27**, and fontations tables no longer implement `Debug` — if we want a table inspector in the editor, we now write it ourselves.
