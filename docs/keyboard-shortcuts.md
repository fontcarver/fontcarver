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

## Spacing and kerning

These gestures belong to the Text tab's canvas and are fixed rather than
remappable; the two Update Metrics commands are ordinary commands and can be
rebound above. `Mod` is Command on macOS and Control elsewhere. The base step is
the first **nudge** distance in Preferences.

| Keys                      | Action                                              |
| ------------------------- | --------------------------------------------------- |
| `[` / `]`                 | Selected side −/+ one base step                     |
| `Shift+[` / `Shift+]`     | …×10                                                |
| `Mod+←` / `Mod+→`         | LSB − / RSB + one unit (`Shift` ×10)                |
| `Mod+Alt+←` / `Mod+Alt+→` | Kern the selected boundary ∓1 (`Shift` ×10)         |
| `Alt+,` / `Alt+.`         | Kern, ignoring the second glyph's group             |
| `Ctrl+,` / `Ctrl+.`       | Kern, ignoring the first glyph's group (macOS)      |
| `,` / `.`                 | Kern at the current scope, spacing-locked mode only |
| `Mod+Alt+M`               | Update Metrics for the current master               |
| `Mod+Alt+Shift+M`         | Update Metrics in all masters                       |

Which side `[` and `]` change — LSB, RSB or both — is chosen in the text bar.
`Mod+[` and `Mod+]` step to the previous and next glyph instead of spacing, so
the bracket keys offer the base step and ×10 only; type an exact value in the
Info box or the spacing matrix for anything larger.
Both-side steps change each side by the same amount and are a single undo entry.
Consecutive steps within 500 ms coalesce into one undo entry.

The spacing keys are disabled in the **spacing locked** kerning mode, and the
unmodified `,`/`.` kerning keys are available only in that mode, because
elsewhere those characters are typed into the run. Control is the primary
modifier on Windows and Linux, so the first-class suppression above is macOS
only; use the Kerning panel's **Rule scope** menu there.

Press **F1** for [context help](user/context-help.md), including while a dialog or
field is focused. F1 itself can be reassigned through this editor. Help remains
accessible from the Help menu even if its key is unbound.
