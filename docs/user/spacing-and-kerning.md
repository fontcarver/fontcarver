# Spacing and kerning

Spacing and kerning are edited in the **Text** tab, where a real shaped run
replaces the guesswork of judging one glyph at a time. The glyph inspector's
**Metrics** section edits the same values for the glyph you have open.

## Metrics formulas

The LSB, RSB and advance of a glyph can be a number or a formula, so a family of
related glyphs stays consistent when its base changes.

| You type  | It means                                                          |
| --------- | ----------------------------------------------------------------- |
| `H`, `=H` | Take the matching sidebearing from `H`                            |
| `=H+10`   | …and add 10; `-`, `*` and `/` work the same way                   |
| `=\|H`    | Take the **opposite** sidebearing of `H` (mirrored)               |
| `=50`     | A constant                                                        |
| `=H@200`  | Measure `H` 200 units above the baseline instead of at its bounds |
| `=+25`    | Offset an **automatically aligned** metric and keep it automatic  |
| `==H`     | Apply to **this layer only**, overriding the glyph's own formula  |

A formula is checked as you type: a typo is reported under the field and nothing
is sent to the document, so the field can be corrected rather than retyped.
Press Escape to go back to the stored formula. Clearing a local `==` formula
restores the glyph-wide one; clearing that removes the formula and leaves the
current numbers in place.

A formula that names a missing glyph, or a chain that refers back to itself, is
reported where the resolved values are shown. FontCarver never loops on a cyclic
formula and never silently invents a value.

The two vertical formula fields in the file format are not resolved; edit the
vertical origin and advance directly in the inspector instead. A document that
already carries one keeps it untouched.

## Keeping formulas and metrics in sync

When the stored numbers no longer match what the formulas resolve to, the glyph
cell shows a warning badge, the inspector says **out of sync**, and the Spacing
matrix marks the row.

- **Glyph → Update Metrics** (`Mod+Alt+M`) applies the resolved values to the
  current master.
- **Glyph → Update Metrics in All Masters** (`Mod+Alt+Shift+M`) does every
  master in one undo step.

Both act on the glyph you are editing, or on the whole Font-tab selection. An
update that would contradict itself — formulas that fix the advance _and_ both
sidebearings to incompatible values — is refused without changing anything.

## The Spacing matrix

Open **Spacing matrix** in the Text tab to get a row per glyph of the current
run. Each of the LSB, RSB and Advance cells takes a number or a formula, so a
whole word can be spaced without leaving the proof. Typing a number over a cell
that held a formula removes that formula in every scope, as one undo step, so the
row cannot be left inheriting the formula it just replaced.

- **Beam** draws a measurement line across the run at a height you choose, and
  prints the real distance between each pair of outlines _on that line_ — the
  measurement that matters for round and diagonal shapes, which their bounding
  boxes hide. Drag the line itself on the canvas, drag the slider, or type the
  height; the distances reappear when you let go. The two beam columns show each
  glyph's own margins at that height.
- **Cut off** hides everything outside a band, so counters and sidebearings can
  be compared without the rest of the letter competing for attention.
- Rows whose formulas disagree with their numbers get their own **Update**
  button.

Saved proof strings live in the **Proof library** panel of the same tab; pick one
to load its text, script, language and direction, or press `↑` and `↓` to step
through them without leaving the spacing keys. See
[Previewing and proofing text](proofing.md).

## Spacing shortcuts

See [Keyboard shortcuts](../keyboard-shortcuts.md#spacing-and-kerning) for the
full table. Choose the side (LSB, RSB or both) in the text bar. `[` and `]`
change it by the first nudge distance in Preferences. Hold Shift for ×10 or Alt
(Option on macOS) for ×100. Hold Alt+Shift for exactly one font unit, regardless
of the nudge distance. For example, with a 2.5-unit base step, Alt+] adds 250
units to each selected side; Alt+Shift+[ subtracts one.
`Mod+[` and `Mod+]` step to the previous and next glyph without changing its
spacing. The Info box and spacing matrix still accept absolute numeric values.

## Kerning modes

The text bar's kerning mode applies to that tab:

- **Kerning off** — the run is shaped without `kern`/`vkrn` and kerning cannot be
  changed. Your stored feature choices are not touched.
- **Kerning on** — edit kerning and spacing.
- **Kerning on, spacing locked** — kerning still applies and can be edited, but
  every spacing control is disabled: the keyboard steps, the sidebearing handles,
  the Info box, the Metrics panel and the Spacing matrix. Use this while kerning
  so a stray key cannot respace a glyph.

## Kerning pairs, groups and exceptions

Click the diamond between two glyphs to select that boundary. The handle is
coloured by the kerning actually applied there — tighter than the plain advance
in the warning colour, looser in green — and the number is printed beside it, so
colour is never the only signal. Every boundary whose rule is an exception is
marked with a ring, selected or not.

Each glyph has one kerning group per side, edited in the **Kerning groups** panel.
A pair with no rule yet is kerned **class to class** by default, which is what
keeps a font's kerning small and consistent; a pair that already has a rule opens
on that rule, so an edit changes what the run is really using. To make an exception for one pair without changing
that default, hold a modifier while kerning:

| Keys                   | Effect                                 |
| ---------------------- | -------------------------------------- |
| `Alt` + `,` / `.`      | Ignore the second glyph's group        |
| `Ctrl` + `,` / `.`     | Ignore the first glyph's group (macOS) |
| `Ctrl+Alt` + `,` / `.` | Kern the two glyphs alone (macOS)      |
| add `Shift`            | Steps of 10                            |

On Windows and Linux, Control is the primary modifier, so the first-class and
both-class variants are chosen in the **Rule scope** menu of the Kerning panel
instead. Unmodified `,` and `.` kern only in the spacing-locked mode; elsewhere
they are typed into the run.

**Show group outlines** overlays up to four other members of each group behind
the selected pair, which is how a collision that only affects one member of a
class is caught.

## The kerning table

**Kerning table** lists every stored record of one master and direction.

- Sort by either side or by value; filter by name with _contains_, _is exactly_,
  _begins with_, _ends with_ or _does not contain_; and narrow to group rules,
  glyph pairs or exceptions only.
- Pick a **Master** in the table to inspect and edit another master without
  moving the proof.
- **Clean Up** removes records that reference missing glyphs or invalid groups,
  and explicit zeros that change nothing. A zero that suppresses a group value is
  a real exception and is kept.
- **Compress** folds pairs into class rules wherever every member of both groups
  already agrees, then removes redundant exceptions. Resolved kerning never
  changes.
- **Copy table** and **Paste table** move kerning between masters. Choose whether
  a conflict overwrites the destination, keeps it, or replaces the whole table.
  The copied record count is shown before you paste.
- Clicking a group name offers two rename scopes: rename the group everywhere
  (membership follows, every master and both directions), or retarget just that
  one record at a different group.

Every change is one undo step, and a bulk command is refused outright if the
document moved after the table was read.

## Vertical spacing and kerning

Switch the text direction to top-to-bottom. Boundaries, the kerning panel and the
kerning table then work on vertical kerning, and the inspector's vertical section
edits the vertical origin, the vertical advance and the top and bottom
sidebearings. Vertical kerning is exported through `vkrn`.
