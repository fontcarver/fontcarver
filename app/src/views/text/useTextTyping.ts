import { useEffect, type RefObject } from 'react'
import { isEditableTarget } from '../../shell/shortcuts'
import { useUiStore } from '../../stores/uiStore'

/** Sends unmodified typing on the text canvas back to its text input. */
export function useTextTyping(
  text: string,
  mode: 'text' | 'edit',
  changeText: (text: string) => void,
  inputRef: RefObject<HTMLInputElement | null>,
) {
  useEffect(() => {
    const typing = (event: KeyboardEvent) => {
      if (
        useUiStore.getState().modalCount > 0 ||
        isEditableTarget(event.target) ||
        event.ctrlKey ||
        event.metaKey ||
        event.altKey
      )
        return
      if (mode !== 'text' || event.key === ' ') return
      if (event.key.length !== 1 && event.key !== 'Backspace') return
      event.preventDefault()
      event.stopImmediatePropagation()
      changeText(
        event.key === 'Backspace'
          ? Array.from(text).slice(0, -1).join('')
          : text + event.key,
      )
      inputRef.current?.focus({ preventScroll: true })
    }
    window.addEventListener('keydown', typing, true)
    return () => window.removeEventListener('keydown', typing, true)
  }, [changeText, inputRef, mode, text])
}
