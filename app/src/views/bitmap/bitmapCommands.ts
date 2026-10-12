/**
 * Strike-editor tools and overlays as commands.
 *
 * Every pixel tool is a command so the Bitmap menu, the command palette and the
 * keymap editor reach exactly what the toolbar does. The shortcuts are plain
 * single keys, which is safe because each command's `when` is false unless a
 * strike editor is mounted: the shell's dispatcher leaves an unbound key to its
 * native action, and text fields never see these keys at all.
 */

import { registerCommands, type Command } from '../../shell/commands'

import {
  bitmapEditorActive,
  useBitmapToolStore,
  type BitmapOverlays,
} from './bitmapToolStore'
import type { PixelTool } from './pixels'

/** The tool palette, in toolbar order, with its single-key shortcut. */
export const PIXEL_TOOL_KEYS: readonly {
  tool: PixelTool
  shortcut: string
  titleKey: string
}[] = [
  { tool: 'pencil', shortcut: 'B', titleKey: 'bitmap.tool.pencil' },
  { tool: 'erase', shortcut: 'E', titleKey: 'bitmap.tool.erase' },
  { tool: 'line', shortcut: 'L', titleKey: 'bitmap.tool.line' },
  { tool: 'rectangle', shortcut: 'R', titleKey: 'bitmap.tool.rectangle' },
  { tool: 'ellipse', shortcut: 'O', titleKey: 'bitmap.tool.ellipse' },
  { tool: 'fill', shortcut: 'G', titleKey: 'bitmap.tool.fill' },
  { tool: 'select', shortcut: 'M', titleKey: 'bitmap.tool.select' },
  { tool: 'eyedropper', shortcut: 'I', titleKey: 'bitmapCanvas.eyedropper' },
  { tool: 'hand', shortcut: 'H', titleKey: 'bitmapCanvas.hand' },
]

/** The overlay switches, in control order. */
export const PIXEL_OVERLAY_KEYS: readonly {
  overlay: keyof BitmapOverlays
  titleKey: string
}[] = [
  { overlay: 'grid', titleKey: 'bitmapCanvas.grid' },
  { overlay: 'outline', titleKey: 'bitmapCanvas.outline' },
  { overlay: 'advanceColumn', titleKey: 'bitmapCanvas.advanceColumn' },
  { overlay: 'fontUnits', titleKey: 'bitmapCanvas.fontUnits' },
]

/** Registers one command per pixel tool and per canvas overlay. */
export function registerBitmapCommands(): void {
  const tools = PIXEL_TOOL_KEYS.map(
    ({ tool, shortcut, titleKey }): Command => ({
      id: `bitmap.tool.${tool}`,
      title: titleKey,
      menu: 'bitmap',
      group: 'tool',
      shortcut,
      when: bitmapEditorActive,
      disabledReason: 'bitmap.reason.noEditor',
      run: () => useBitmapToolStore.getState().setTool(tool),
    }),
  )
  const overlays = PIXEL_OVERLAY_KEYS.map(({ overlay, titleKey }): Command => ({
    id: `bitmap.overlay.${overlay}`,
    title: titleKey,
    menu: 'bitmap',
    group: 'overlay',
    when: bitmapEditorActive,
    disabledReason: 'bitmap.reason.noEditor',
    run: () => useBitmapToolStore.getState().toggleOverlay(overlay),
  }))
  registerCommands([...tools, ...overlays])
}
