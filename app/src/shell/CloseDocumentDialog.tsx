import { selectSummary, useSessionStore } from '../stores/sessionStore'
import { useUiStore } from '../stores/uiStore'

import { CloseDocumentPrompt } from './CloseDocumentPrompt'

/**
 * Save / Don't Save / Cancel prompt shown when a dirty document is closed.
 * Saving closes the document only once the kernel reports the write succeeded,
 * asking for a `.fcf` path first when the document has none.
 *
 * The body is mounted on demand so that its modal chrome — the modal count, the
 * focus trap and the control focus returns to — exists only while the prompt is
 * on screen.
 */
export function CloseDocumentDialog() {
  const prompt = useUiStore((state) => state.closePrompt)
  const summary = useSessionStore(selectSummary(prompt?.doc ?? null))

  if (!prompt || !summary) return null
  return <CloseDocumentPrompt prompt={prompt} summary={summary} />
}
