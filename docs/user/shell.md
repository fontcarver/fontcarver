# Windows, tabs, workspaces and project files

This page covers the frame around the editors: how documents map to windows, how
the tab strip and the panels work, how a named arrangement is saved and restored,
how to read and move through the document's history, and how the text-first
project format and crash recovery store your work.

## One window per document

On the desktop, each open document owns one window. Opening a second document
from a window that already holds one moves it into its own window, so a window
title and a document are the same thing. **Window → Open document window**
reopens or focuses the window of the active document.

In the browser development build there are no operating-system windows to give
out, so extra views open as named browser popups instead. A browser that blocks
popups reports that the window could not be opened and the view stays where it
was.

## The tab strip

Every document's tabs form one run in the strip, and its **Overview** tab is
pinned first — one Overview per document, however many times the document's tabs
are reopened. Opening a file that is already open opens it a second time as its
own document, so that a saved file can be compared with the copy you have been
editing; both documents are named after the file, and each has its own Overview. The Overview stands for the document itself: it has no close
button, and **Close Tab** (`Mod+Shift+W`) is unavailable while it is selected —
use **File → Close Font** (`Mod+W`) to close the document and all of its tabs.

| Action                       | Shortcut                      |
| ---------------------------- | ----------------------------- |
| Overview                     | `Mod+Alt+1`                   |
| Second to ninth tab          | `Mod+Alt+2` … `Mod+Alt+9`     |
| Next / previous tab          | `Ctrl+Tab` / `Ctrl+Shift+Tab` |
| Next / previous tab (arrows) | `Mod+Alt+→` / `Mod+Alt+←`     |
| Close tab                    | `Mod+Shift+W`                 |

The numbered shortcuts count along the active document's own run of tabs, in the
order the strip shows them, and reach every kind of editing tab — Glyph, Font
Info, Features and Bitmap alike. Each tab button also publishes its binding to
assistive technology, so a screen reader announces how to reach it.

`Mod` is Command on macOS and Control on Windows and Linux. The tab-cycling
`Ctrl+Tab` bindings use the physical Control key on macOS too. **Help → Keyboard
shortcuts** lists the current bindings and lets you change them.

## Tearing views off into their own windows

An editing tab can be moved into its own window with the ↗ button on the tab, by
dragging the tab out of the strip, or with **Window → Detach active tab**. The
left sidebar, the right Inspector and the Edit tab's preview strip each have the
same ↗ button and a **Window → Detach …** command. A detached view shares the
document with its main window: edits, undo and saving all apply to the same
document.

A detached sidebar or Inspector torn off an Edit tab shows that glyph's live
content and follows the glyph the owning window moves to, so stepping through
glyphs in the main window updates the panel in its own window.

**Return to document window** in a detached window puts the view back where it
came from and closes the window. Closing a detached window directly does the
same thing, so nothing is lost either way.

## Panels

The left sidebar follows the active tab: glyph filters on the Overview, the
glyph's layers and glyph list in an Edit tab. The right Inspector is a scrolling
stack of sections — the sections of the active editor, the document facts, the
Overview's display settings and the document History. Each section remembers
whether it is open, and both docks can be resized by dragging their divider or
hidden with `Mod+1` (sidebar) and `Mod+2` (Inspector).

## The bottom preview strip

An Edit tab shows a preview strip under the canvas.

- Drag the divider along its top edge to resize it, or focus it and use
  `↑`/`↓`, `Home` and `End`. The height is remembered per tab and saved with the
  workspace.
- `Mod+3` (**View → Text preview strip**) shows or hides the strip.
- `Mod+Alt+P` (**View → Collapse Preview Strip**) collapses it to its title bar
  and expands it again.

## Saved workspaces

The shell remembers each document's latest arrangement — its tabs and which one
was selected, dock visibility and widths, the Inspector's open sections, the
preview strip's state and height, and which views are in their own windows — and
restores it the next time that document is opened.

**Window → Workspaces…** saves the current arrangement under a name, applies a
saved one, or deletes it. Applying an arrangement also reopens the windows it had
detached; a glyph window whose glyph no longer exists is skipped rather than
opened on a different glyph.

Named workspaces are stored with the document:

- a `.fcfp` project keeps them in `workspaces.json` inside the project
  directory, so they travel with it through Git;
- a `.fcf` or `.fcf.gz` document keeps them in `<name>.fcf.workspaces.json`
  beside it.

A document with no FontCarver storage yet — an imported binary font, or one that
has never been saved — keeps its arrangements on this computer only; the
Workspaces dialog says which of the two applies. Saving it as a FontCarver
document or project moves them into the project on the next save.

## Document history

The Inspector's **History** section lists the document's undo timeline, newest
first. Each row is one gesture — a drag, a typed field you committed, a dialog
you applied — and says what it changed: the glyphs, the layers inside them, the
bitmap strikes, or that it was font-wide. A layer the font names is named; one
with no name of its own is described by what it is for — master, background,
intermediate, alternate, smart, backup — rather than by its internal identifier.
An edit to one glyph's color, SVG or math record names that glyph as well as the
font-wide table it lives in. The current position is marked, and
rows below it are the ones undo has already passed.

Selecting a row moves the document to the state just after that operation; the
last row returns it to how the document was opened. This is the same work as
pressing `Mod+Z` or `Mod+Shift+Z` that many times. If the timeline moved while
the list was open — another window edited the document, or an old entry was
dropped to stay inside the history memory budget — the jump is refused, the
document is left exactly as it was, and the list reloads.

**Edit → History** opens the Inspector and this section.

## Text-first projects (`.fcfp`)

**File → Save As** with a `.fcfp` name writes a directory project instead of a
single file:

```
Family.fcfp/
  project.json       # everything except glyph outlines, pretty-printed JSON
  glyphs/            # one pretty-printed JSON file per glyph revision
  workspaces.json    # named workspaces, when any are saved
  .fontcarver.lock   # advisory lock; safe to ignore in Git
```

The output is deterministic: saving a document that has not changed rewrites no
byte, and every text file ends with a newline. Editing one glyph replaces that
glyph's file and the manifest entry that points at it and leaves every other
file untouched, so a commit shows the glyphs you actually worked on. Glyph file
names are content digests, which is why they change when the glyph does; the
manifest keeps the glyph order, so file names never carry meaning.

You can edit a glyph file by hand. FontCarver reads back what you wrote and
renames the file on the next save; you never have to recompute a digest. A
manifest that points at a missing file, a duplicate, a symlink, or JSON whose
glyph name disagrees with its entry is refused rather than half-loaded.

Add `*.fcfp/.fontcarver.lock` and `*.fcfp/.fontcarver-*.tmp` to `.gitignore`.

## Crash recovery

While a document has unsaved changes, FontCarver writes a complete snapshot of
it every minute to the recovery directory inside its configuration folder
(`$XDG_DATA_HOME/fontcarver/autosave` on Linux by default). The snapshot records
the path of the document it belongs to, including a `.fcfp` directory, so it
stays readable even when the project it came from was damaged mid-save.

The next launch lists every snapshot that is newer than its document and offers
to recover it. Recovering opens the unsaved work as a modified document that
keeps the original path — a banner in the Overview says where it came from — so
saving it overwrites the document the snapshot belongs to. Discarding asks once,
then deletes the snapshot. Saving the document makes its snapshot obsolete, and
it is no longer offered.
