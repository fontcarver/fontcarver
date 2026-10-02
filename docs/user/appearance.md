# Appearance, language and accessibility

This page covers the two settings that change how the whole interface looks and
reads — the color theme and the interface language — and what FontCarver
guarantees for keyboard and screen-reader use.

## Light and dark

**View → Light Theme / Dark Theme / Match System Theme** switches the palette
immediately; so does **Theme** in **Preferences → General**. Both write the same
preference, so the menu and the dialog never disagree, and the choice is stored
with your other preferences and restored the next time you start.

**Match System Theme** follows the operating system while it is selected: if your
desktop switches to dark in the evening, the interface follows without a restart.

Every surface has a designed dark palette rather than an inverted light one.
Three palettes are kept apart on purpose:

- **The interface palette** paints the menu bar, tabs, panels, dialogs and the
  font overview grid.
- **The glyph-canvas palette** paints the outline editor and the shaped-text
  canvas: the ground, outlines, nodes, handles, metric lines, guides, anchors,
  alignment zones and the cursor readout. Nodes keep their shape as well as their
  color (square for a corner, circle for a smooth node), so the canvas never
  relies on color alone to say what something is.
- **The pixel-canvas palette** paints the strike editor and the bitmap
  waterfall. Its paper and ink are not the panel background and the label color:
  a pixel surface that matched the chrome would read as chrome.

Text contrast is measured, not estimated. Interface text meets a 4.5:1 ratio
against every background it is drawn on in both themes, and the canvas markers
meet 3:1 against their canvas ground. The metric lines, the sidebearing bands,
the alignment-zone fills, the background layer and the other-master overlay are
deliberately quieter than that: they are reference washes behind the drawing, and
they must not compete with the outline you are editing.

The text preview strip is a proof surface and has its own **light/dark switch**
in its header, independent of the interface theme: a proof is judged on paper
white or on a dark ground, whichever you are designing for, whatever the
surrounding interface looks like.

## Interface language

FontCarver ships English and Simplified Chinese. Switch with **Language** in
**Preferences → General**, or straight from **View → English Interface** /
**中文界面 (Chinese Interface)** — the View menu entry exists so that a reader
who cannot read the current language does not have to find the right row of a
dialog first. Both routes are also in the command palette.

The change takes effect immediately, with no reload and no loss of unsaved work,
and it is remembered. The document's own data is never translated: glyph names,
feature code, axis tags and file paths are identifiers and stay as they are.
The page's `lang` attribute follows the choice, so a screen reader switches
voice with the interface.

Font-unit numbers are formatted the same way in both languages, because a
coordinate typed in one language has to be readable in the other.

## Settings in a zoomed or short window

Preferences and Keyboard Shortcuts keep their bottom buttons visible while
their contents scroll. In **Preferences → Editor**, the three nudge fields wrap
when space is tight. Scroll inside the form, or use Tab to bring the next
control into view. The shortcut list also scrolls independently; search by
command name, ID or binding to reach an action directly.

## Keyboard and screen readers

- Every interactive control is reachable with Tab, and the tool strip, menus, the
  command palette and the glyph grid are all operable without a pointer. See
  [keyboard shortcuts](../keyboard-shortcuts.md) for the bindings and how to
  change them.
- Icon-only buttons carry an accessible name, and a control that explains itself
  carries that explanation as a description a screen reader reads out — not only
  as a tooltip a pointer has to hover for.
- Every dialog names itself, takes focus when it opens, keeps Tab inside itself
  while it is open, closes on Escape, and returns focus to the control you came
  from. While a dialog is open the global shortcuts stand down, so a key cannot
  act on the document behind it.
- Press **F1** for [context help](context-help.md) on whatever is focused or
  under the pointer, including inside a dialog. Resting the pointer on a control
  for a moment shows the same help as a small bubble that never takes focus.
- Menus and the command palette state why an unavailable command is unavailable
  rather than hiding it.
- The interface honours `prefers-reduced-motion`: with it set, transitions and
  animations are reduced to a single millisecond.
