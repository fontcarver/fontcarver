import { useEffect, useRef, useState } from 'react'

import { useUiStore } from '../stores/uiStore'

/**
 * Selector of the controls a focus trap may land on. Matches the one the
 * `Dialog` primitive uses, so a hand-built modal traps focus the same way.
 */
const FOCUSABLE =
  'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])'

/**
 * The modal chrome every dialog owes its reader, for the dialogs that build
 * their own markup instead of using the {@link Dialog} primitive.
 *
 * Three of them exist — the close prompt, the path prompt and the command
 * palette — because each needs a shape the primitive does not offer. They still
 * owe the same four things, and this supplies all four: the dialog counts itself
 * as a modal (so the global shortcut dispatcher stands down and `Mod+W` behind a
 * "save before closing?" prompt cannot close the document the prompt is asking
 * about), focus moves inside on mount, Tab is trapped, and Escape cancels.
 *
 * Mount the component that calls this only while its dialog is on screen. The
 * element to hand focus back to is captured during that first render, because
 * React applies an `autoFocus` control during the commit: read from an effect,
 * the active element would already be a control inside the dialog, and focus
 * would be "restored" to a node that is about to be removed.
 *
 * Spread the result onto the element that carries `role="dialog"`.
 */
export function useModalChrome(onCancel: () => void): {
  ref: React.RefObject<HTMLDivElement | null>
  onKeyDown: (event: React.KeyboardEvent) => void
} {
  const ref = useRef<HTMLDivElement>(null)
  const [restore] = useState<HTMLElement | null>(() =>
    document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null,
  )

  useEffect(() => {
    const ui = useUiStore.getState()
    ui.openModal()
    return ui.closeModal
  }, [])

  useEffect(() => {
    const dialog = ref.current
    // An `autoFocus` control inside the dialog wins; otherwise the first
    // control takes focus so the trap always has something inside it.
    if (dialog && !dialog.contains(document.activeElement))
      dialog.querySelector<HTMLElement>(FOCUSABLE)?.focus()
    return () => restore?.focus()
  }, [restore])

  const trapTab = (event: React.KeyboardEvent): void => {
    const items = [
      ...(ref.current?.querySelectorAll<HTMLElement>(FOCUSABLE) ?? []),
    ]
    if (items.length === 0) return
    const first = items[0]
    const last = items[items.length - 1]
    const active = document.activeElement
    if (
      event.shiftKey &&
      (active === first || !ref.current?.contains(active))
    ) {
      event.preventDefault()
      last?.focus()
    } else if (!event.shiftKey && active === last) {
      event.preventDefault()
      first?.focus()
    }
  }

  return {
    ref,
    onKeyDown: (event: React.KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        event.stopPropagation()
        onCancel()
        return
      }
      if (event.key === 'Tab') trapTab(event)
    },
  }
}
