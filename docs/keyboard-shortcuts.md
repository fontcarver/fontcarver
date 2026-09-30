# Keyboard shortcuts

Open **Help → Keyboard Shortcuts** to search by command name, stable action ID or
binding. Edit a field using notation such as `Mod+Shift+P`; `Mod` means Command
on macOS and Control on Windows/Linux. Clear a field to disable that binding,
or choose **Restore** beside it to recover the shipped default.

Changes are drafts until **Apply**. **Cancel** discards the draft. Conflicting
bindings are listed together and prevent Apply until resolved. Context-exclusive
Font-grid and Glyph-editor Select All bindings may share a key. Fixed canvas
pan/preview and nudge gestures also participate in conflict checks. Tab and Escape
remain available for focus traversal and dismissal. Text inputs, composition,
native button activation, and modal dialogs keep ownership of their keys.

**Load preset** explicitly replaces the draft with FontCarver defaults or the
supported Glyphs, FontLab or RoboFont subset. See [preset mappings and
sources](keymap-presets.md). Presets never activate on startup or overwrite a
saved user map automatically.

**Export JSON** saves the current valid draft. **Import keymap JSON** stages a
versioned file for review; it never executes content. Unknown action IDs are
shown and must be removed before Apply. Files are limited to 256 KiB and 2,000
bindings. A sample:

```json
{
  "version": 1,
  "bindings": {
    "file.open": "Mod+K",
    "tool.pen": "U",
    "tool.knife": null
  }
}
```

Only explicit changes are exported; actions omitted from the file use their
shipped defaults. Shortcuts persist in this app/browser origin's local storage
and survive reload. If saving fails, the editor reports the failure and retains
the active keymap. Export a file to transfer a map between browsers or devices.

The menu, command palette and tool strip show the active bindings. Tools retain
tap-to-select and hold-to-borrow behavior after remapping. Space/pan, arrow
nudging, text metrics and navigation gestures keep their existing behavior and
are described separately from configurable commands.

Press **F1** for [context help](user/context-help.md), including while a dialog or
field is focused. F1 itself can be reassigned through this editor. Help remains
accessible from the Help menu even if its key is unbound.
