# Shortcut compatibility presets

The shortcut editor offers Glyphs, FontLab and RoboFont presets. Each preset
replaces the current custom map with the supported subset below. Other actions
use FontCarver's defaults. Export the map before switching presets if you want
to keep your own changes. `Mod` means Command on macOS and Control elsewhere.

These presets change shortcuts. They do not reproduce another application's
tools or interaction model. FontCarver tools still use tap to select and a
200 ms hold to borrow until release. The palette displays the effective binding;
an unbound tool remains available by clicking it.

| FontCarver tool | Glyphs  | FontLab | RoboFont |
| --------------- | ------- | ------- | -------- |
| Select          | V       | A       | 1        |
| Pen             | P       | P       | 2        |
| Pencil          | B       | N       | default  |
| Knife           | E       | J       | 3        |
| Rectangle       | F       | I       | default  |
| Ellipse         | Shift+F | O       | default  |
| Measure         | L       | default | 4        |
| Rotate          | R       | R       | default  |
| Scale           | S       | S       | default  |
| Zoom            | Z       | default | default  |
| Hand            | H       | default | default  |
| Text            | T       | T       | default  |

The Glyphs preset includes New (`Mod+N`), Save (`Mod+S`) and Save As
(`Mod+Shift+S`). FontLab and RoboFont also include Open (`Mod+O`), Undo
(`Mod+Z`) and Redo (`Mod+Shift+Z`). Export is `Mod+E` for FontLab and
`Mod+Alt+G` for RoboFont. RoboFont also includes Close (`Mod+W`).

Sources checked 2026-09-30:

- [Glyphs Handbook](https://handbook.glyphsapp.com/single-page/) documents its
  tool keys and these file commands. Glyphs shares E between erasing tools and
  F between primitives, with Shift+F cycling primitives. FontCarver assigns E
  directly to Knife and F / Shift+F directly to Rectangle / Ellipse.
- FontLab 8 [Draft & draw](https://help.fontlab.com/fontlab/8/whats-new/whats-new-02-draft-draw/),
  [Edit & refine](https://help.fontlab.com/fontlab/8/whats-new/whats-new-03-edit-refine/)
  and [Explore & prepare](https://help.fontlab.com/fontlab/8/whats-new/whats-new-01-explore-prepare/)
  establish the listed tool mappings. Select maps to Contour (A), not Element
  (V). File/edit commands are sourced from the official
  [FontLab 7 shortcut table](https://help.fontlab.com/fontlab/7/manual/Keyboard-shortcuts/);
  these commands were not separately verified in a running FontLab 8 instance.
  FontLab's G controls Guides, while its
  [Z and X keys zoom immediately](https://help.fontlab.com/fontlab/8/tutorials/calfonts/0.%20Installation%20and%20Set%20Up/00b-1%20Customize%20Your%20Drawing%20Window/).
  Those functions do not match FontCarver's Measure or Zoom tools, so those
  bindings and Hand remain at FontCarver defaults.
- RoboFont's [Glyph View preferences](https://doc.robofont.com/documentation/reference/workspace/preferences-window/glyph-view/)
  document bare 1–4 for tools. The
  [application menu](https://doc.robofont.com/documentation/reference/workspace/application-menu/)
  gives their order (Edit, Bézier, Slice, Measurement). Its
  [keyboard shortcut reference](https://robofont.com/documentation/reference/workspace/keyboard-shortcuts/)
  documents the listed file/edit commands. The separate Command+1–4 menu
  shortcuts are not imported: FontCarver uses the bare-number tool bindings.

Custom bindings match their modifiers exactly. Unchanged FontCarver default
letter keys also accept Shift, except where an exact binding takes priority
(for example Shift+F). Preset bindings are explicit and therefore use exact
modifiers. Text fields and open dialogs keep ownership of typing.

The FontLab subset unbinds FontCarver’s Annotation tool so that its default A
does not conflict with FontLab-style Select. The annotation tool remains
available from the tool strip and can be assigned another key in the editor.

## Presets and conflicts

Loading a preset replaces the whole draft, including any conflicts you had
deliberately allowed: a preset is a clean starting point, not a patch. The three
subsets are checked against the shipped defaults and against each other by
`src/shell/keymapIntegration.test.ts`, which fails if a preset introduces a
conflict in a context where both bindings are live, so none of them needs an
allowance to be applied.

The strike editor's pixel tools use plain single keys (B, E, L, R, O, G, M, I, H)
and the outline editor's tools use their own (V, P, B, E, F, ⇧F, L, R, S, Z, H, T).
They overlap on paper and never in practice: a pixel tool is live only while a
Bitmap tab is mounted and an outline tool only on an Edit tab, so the editor
treats them as different contexts and reports no conflict. Rebinding one does not
free the other.
