/**
 * Tool keys with FontLab's tap/hold grammar (design spec §5).
 *
 * A tap switches the tool; holding the key for 200 ms borrows the tool until
 * the key comes back up, which the strip shows with a dotted outline. The
 * listener is installed on the window rather than on the canvas so a tool key
 * works wherever focus is inside the Edit tab — except in a text field, where
 * "v" means "v".
 */

import { useEffect, useRef } from 'react'

import { HOLD_BORROW_MS, toolForKey } from '../../editor/tools/registry'
import { isEditableTarget } from '../../shell/shortcuts'
import { useEditorStore } from '../../stores/editorStore'

/** Installs the tool-key handling while a glyph tab is mounted. */
export function useToolShortcuts(enabled: boolean): void {
  const pending = useRef<{
    key: string
    timer: number
    borrowed: boolean
  } | null>(null)

  useEffect(() => {
    if (!enabled) return
    const store = useEditorStore.getState

    const clear = () => {
      const held = pending.current
      if (!held) return
      clearTimeout(held.timer)
      pending.current = null
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.ctrlKey || event.metaKey || event.altKey || event.repeat) return
      if (isEditableTarget(event.target)) return
      const tool = toolForKey(event.key, event.shiftKey)
      if (!tool || pending.current) return
      event.preventDefault()
      const timer = setTimeout(() => {
        if (!pending.current) return
        pending.current = { ...pending.current, borrowed: true }
        store().borrowTool(tool)
      }, HOLD_BORROW_MS) as unknown as number
      pending.current = { key: event.key.toLowerCase(), timer, borrowed: false }
    }

    const onKeyUp = (event: KeyboardEvent) => {
      const held = pending.current
      if (!held || held.key !== event.key.toLowerCase()) return
      clear()
      const tool = toolForKey(event.key, event.shiftKey)
      if (!tool) return
      if (held.borrowed) store().borrowTool(null)
      else store().setTool(tool)
    }

    window.addEventListener('keydown', onKeyDown)
    window.addEventListener('keyup', onKeyUp)
    return () => {
      clear()
      store().borrowTool(null)
      window.removeEventListener('keydown', onKeyDown)
      window.removeEventListener('keyup', onKeyUp)
    }
  }, [enabled])
}
