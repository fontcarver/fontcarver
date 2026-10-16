import { TOOLS } from '../editor/tools/registry'
import { listCommands, type Command } from './commands'
import type { KeymapTarget } from './keymap'

/** Editor gestures have lifecycle semantics and are not command bindings. */
export const FIXED_SHORTCUT_IDS = new Set([
  'view.toggleFill',
  'text.kerning.ArrowLeft',
  'text.kerning.ArrowRight',
])

function scopeOf(command: Command): string {
  // The strike editor's tools and overlays are live only while a Bitmap tab is
  // mounted, and the glyph tools only on an Edit tab, so their single-key
  // bindings are mutually exclusive and must not read as conflicts.
  if (command.id.startsWith('bitmap.') && command.id !== 'bitmap.open')
    return 'bitmap'
  if (
    ['edit.selectAll', 'edit.selectNone', 'edit.copyGlyphNames'].includes(
      command.id,
    )
  )
    return 'font'
  if (
    command.id.startsWith('path.') ||
    command.id.startsWith('layer.') ||
    command.id.startsWith('view.glyph.') ||
    command.id.startsWith('tool.') ||
    [
      'edit.selectAllPoints',
      'edit.deselectPoints',
      'edit.deletePoints',
      'edit.deletePointsBreak',
      'glyph.previous',
      'glyph.next',
      'view.zoomFit',
      'view.toggleInfoBox',
      'view.toggleSnapping',
    ].includes(command.id)
  )
    return 'glyph'
  return 'global'
}

/** Every editable command and tool, using stable action IDs rather than labels. */
export function keymapTargets(
  commands: readonly Command[] = listCommands(),
): KeymapTarget[] {
  return [
    ...commands
      // Tools have palette commands, but their binding belongs to the tap/hold
      // handler. Use the tool table below as their single editable row.
      .filter(
        (command) =>
          !FIXED_SHORTCUT_IDS.has(command.id) &&
          !command.id.startsWith('tool.'),
      )
      .map((command) => ({
        id: command.id,
        title: command.title,
        shortcut: command.shortcut,
        scope: scopeOf(command),
      })),
    ...TOOLS.map((tool) => ({
      id: `tool.${tool.id}`,
      title: tool.labelKey,
      shortcut: `${tool.shift ? 'Shift+' : ''}${tool.key.toUpperCase()}`,
      scope: 'glyph',
    })),
  ]
}

/** Fixed canvas gestures also participate in customized-binding conflict checks. */
export function reservedKeymapTargets(): KeymapTarget[] {
  return [
    {
      id: 'gesture.panPreview',
      title: 'shortcuts.fixedPan',
      shortcut: 'Space',
      scope: 'glyph',
    },
    ...['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].flatMap((key) =>
      ['', 'Shift+', 'Mod+', 'Mod+Shift+'].map((modifier) => ({
        id: `gesture.${modifier}${key}`,
        title: 'shortcuts.fixedNudge',
        shortcut: `${modifier}${key}`,
        scope: 'glyph',
      })),
    ),
    ...['[', ']'].flatMap((key) =>
      ['', 'Shift+', 'Alt+', 'Alt+Shift+'].map((modifier) => ({
        id: `gesture.spacing.${modifier}${key}`,
        title: 'shortcuts.fixedSpacing',
        shortcut: `${modifier}${key}`,
        scope: 'glyph',
      })),
    ),
  ]
}
