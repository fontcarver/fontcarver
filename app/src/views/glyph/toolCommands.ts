/**
 * One command per tool, so the command palette reaches every tool.
 *
 * Design spec §13: nothing is more than two clicks or one shortcut away, and
 * the palette lists everything with its shortcut. The tool strip and the tool
 * keys already cover the first two routes; this is the third.
 *
 * The commands deliberately carry no `shortcut` of their own. Tool keys have a
 * tap/hold grammar that needs both the key down and the key up
 * (`useToolShortcuts`), which a one-shot command dispatch cannot express, so
 * the key stays with that layer and the palette shows the live binding as the
 * command's detail instead.
 */

import { i18n } from '../../i18n'
import { registerCommands, type Command } from '../../shell/commands'
import { formatShortcut } from '../../shell/shortcuts'
import { toolShortcut } from '../../shell/toolShortcuts'
import { useEditorStore } from '../../stores/editorStore'
import { TOOLS } from '../../editor/tools/registry'

/** Whether an Edit tab is showing a glyph the tools could act on. */
function hasGlyph(): boolean {
  return useEditorStore.getState().activeGlyph !== null
}

/** Registers a palette command for every tool. Safe to call more than once. */
export function registerToolCommands(): void {
  registerCommands(
    TOOLS.map((tool): Command => ({
      id: `tool.${tool.id}`,
      title: tool.labelKey,
      group: 'tools',
      when: hasGlyph,
      disabledReason: 'cmd.reason.noGlyphLayer',
      detail: () => {
        const binding = toolShortcut(tool.id)
        const hint = tool.hintKey ? i18n.t(tool.hintKey) : undefined
        const key = binding ? formatShortcut(binding) : undefined
        return [key, hint].filter(Boolean).join(' · ') || undefined
      },
      run: () => {
        const store = useEditorStore.getState()
        store.borrowTool(null)
        store.setTool(tool.id)
      },
    })),
  )
}
