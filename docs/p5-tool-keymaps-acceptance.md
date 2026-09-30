# Configurable tool shortcuts and compatibility presets acceptance

Date: 2026-09-30. Scope and interfaces are recorded in
`superpowers/plans/tasks/p5-108-tool-keymaps.md`.

Implemented live keymap overrides for all 12 tools, null unbinding, exact custom
modifiers and the original Shift fallback for unchanged defaults. Tap/hold keeps
the selected tool while borrowing another; physical-key release and window blur
end the temporary state. A command's prevented keyboard event cannot also
select a tool. Tool palette labels react to saved bindings and expose localized
tap/hold hints and the glyph help topic.

`KEYMAP_PRESETS` provides Glyphs, FontLab and RoboFont supported subsets. The
official sources and intentional differences are documented in
[keymap-presets.md](keymap-presets.md). No third-party code was copied and no
dependency was added.

New tests failed before implementation for live overrides, command cooperation,
modified-key release and blur cleanup. Final focused command:

```text
TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5-tool-keymaps \
  pnpm --dir app exec vitest run src/shell/toolShortcuts.test.ts \
  src/shell/keymapPresets.test.ts src/views/glyph/useToolShortcuts.test.tsx \
  src/views/glyph/ToolPalette.test.tsx --maxWorkers=1

Test Files  4 passed (4)
Tests       21 passed (21)
Duration    48.87s
```

Scoped ESLint passed for the eight owned TypeScript/TSX files; Prettier formatted
those files. A preliminary invocation with an extra `--` accidentally started
the full suite and was interrupted; it is not counted as passing verification.
The parent owns integrated TypeScript, full regression and headless browser
verification of the shortcut editor, palette help and live labels.

No server or browser process was started by this task. Removed 5.7 MiB of owned
Vitest transformation cache from task scratch at completion. Artifact checks passed at start, after the interrupted
run and before completion; final values are reported to the parent because
other concurrent tasks continue to change shared build sizes.
