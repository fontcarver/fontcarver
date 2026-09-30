import { useEffect } from 'react'
import { createPortal } from 'react-dom'

import { ContextHelpArticle } from './ContextHelpArticle'
import { installContextHelp, useContextHelpStore } from './contextHelp'

/** Contextual help can be mounted once alongside the shell's other dialogs. */
export function ContextHelpDialog() {
  useEffect(installContextHelp, [])
  const context = useContextHelpStore((state) => state.context)
  if (!context) return null
  return createPortal(<ContextHelpArticle context={context} />, document.body)
}
