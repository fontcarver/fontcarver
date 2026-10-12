import { useEffect } from 'react'
import { createPortal } from 'react-dom'

import { ContextHelpArticle } from './ContextHelpArticle'
import { installContextHelp, useContextHelpStore } from './contextHelp'
import { QuickHelp } from './QuickHelp'

/**
 * Contextual help can be mounted once alongside the shell's other dialogs.
 *
 * It owns both halves of the help surface: the non-modal Quick Help bubble that
 * hover and focus raise, and the full article that F1 and the bubble's own
 * action open above the work.
 */
export function ContextHelpDialog() {
  useEffect(installContextHelp, [])
  const context = useContextHelpStore((state) => state.context)
  return (
    <>
      <QuickHelp />
      {context
        ? createPortal(<ContextHelpArticle context={context} />, document.body)
        : null}
    </>
  )
}
