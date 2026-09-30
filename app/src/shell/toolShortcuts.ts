import { TOOLS, toolById, type ToolId } from '../editor/tools/registry'
import { useKeymapStore } from '../stores/keymapStore'

import { shortcutFor, type KeymapOverrides } from './keymap'
import { isMacPlatform, matchesShortcut, parseShortcut } from './shortcuts'

/** Effective binding for a tool, including explicit unbinding. */
export function toolShortcut(
  tool: ToolId,
  overrides: KeymapOverrides = useKeymapStore.getState().overrides,
): string | null {
  const definition = toolById(tool)
  const fallback = `${definition.shift ? 'Shift+' : ''}${definition.key.toUpperCase()}`
  return shortcutFor(`tool.${tool}`, fallback, overrides)
}

/** Resolve the latest tool bindings without reinstalling keyboard listeners. */
export function toolForShortcut(
  event: KeyboardEvent,
  mac: boolean = isMacPlatform(),
): ToolId | null {
  const overrides = useKeymapStore.getState().overrides
  for (const tool of TOOLS) {
    const binding = toolShortcut(tool.id, overrides)
    const parsed = binding && parseShortcut(binding)
    if (parsed && matchesShortcut(event, parsed, mac)) return tool.id
  }
  // Keep the original Shift-insensitive letter shortcuts only for unchanged
  // defaults. Exact custom bindings, including Shift bindings, take priority.
  if (event.shiftKey) {
    for (const tool of TOOLS) {
      if (tool.shift || Object.hasOwn(overrides, `tool.${tool.id}`)) continue
      const parsed = parseShortcut(tool.key)
      if (parsed && matchesShortcut(event, { ...parsed, shift: true }, mac)) {
        return tool.id
      }
    }
  }
  return null
}
