import type { ReactNode } from 'react'

import styles from './fontInfo.module.css'

export interface SectionBodyProps {
  /** Section heading, shown only while a search spans every section. */
  heading?: string
  /** Whether the section has anything to show. */
  hasContent: boolean
  /** Message shown when the section is empty and no heading is requested. */
  empty: string
  children: ReactNode
}

/**
 * One Font Info section body.
 *
 * While the search spans every section, a section with no matches renders
 * nothing at all — heading included — so the results read as one list instead
 * of four "no matches" notices.
 */
export function SectionBody({
  heading,
  hasContent,
  empty,
  children,
}: SectionBodyProps) {
  if (!hasContent)
    return heading ? null : <p className={styles.empty}>{empty}</p>
  return (
    <>
      {heading ? <h2 className={styles.sectionTitle}>{heading}</h2> : null}
      {children}
    </>
  )
}
