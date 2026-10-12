/**
 * The Pixel tool's handover to the strike editor.
 *
 * Pixel drawing is not an outline gesture: a strike is a different document
 * surface with its own chrome (which size am I editing?), so the tool strip's
 * Pixel entry is a *door*, not a state machine. Choosing it — by click, by its
 * shortcut, or by holding the shortcut — persists whatever the outline editor
 * has open, puts the previous tool back so the Edit tab is not left holding a
 * tool that does nothing, and opens the bitmap workspace.
 */

import { useEffect, useRef } from 'react'

import type { DocId } from '../../api'
import type { ToolId } from '../../editor/tools/registry'
import { reportError } from '../../shell/commands'
import { openBitmapTab } from '../../shell/tabActions'
import { selectActiveTool, useEditorStore } from '../../stores/editorStore'

/** The tool the strip goes back to when Pixel was chosen from it. */
export function toolAfterPixel(previous: ToolId): ToolId {
  return previous === 'pixel' ? 'select' : previous
}

/**
 * Sends the Pixel tool to the strike editor and restores the previous tool.
 *
 * Exported for the regression test, which drives the same path the effect does
 * without mounting the whole Edit tab.
 */
export function handOverToStrikeEditor(doc: DocId, previous: ToolId): void {
  const store = useEditorStore.getState()
  store.borrowTool(null)
  store.setTool(toolAfterPixel(previous))
  void openBitmapTab(doc).catch(reportError)
}

/** Watches the active tool and performs the handover when Pixel is chosen. */
export function usePixelHandover(doc: DocId, enabled: boolean): void {
  const tool = useEditorStore(selectActiveTool)
  const previous = useRef<ToolId>('select')
  useEffect(() => {
    if (!enabled) return
    if (tool !== 'pixel') {
      previous.current = tool
      return
    }
    handOverToStrikeEditor(doc, previous.current)
  }, [doc, enabled, tool])
}
