import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocId } from '../../api'
import { useDocumentStore } from '../../stores/documentStore'

import styles from './fontView.module.css'

/** Delay between the last keystroke and the filtered request, in milliseconds. */
export const SEARCH_DEBOUNCE_MS = 150

export interface GlyphSearchProps {
  doc: DocId
  /** The query currently in the filter, so external changes reach the field. */
  value: string | null
}

/**
 * The glyph search field. The kernel matches a glyph name, a `U+XXXX` scalar,
 * a `uniXXXX` name or a single character, so the field passes the text through
 * unchanged and only debounces it.
 */
export function GlyphSearch({ doc, value }: GlyphSearchProps) {
  const { t } = useTranslation()
  const setFilter = useDocumentStore((state) => state.setFilter)
  // `null` means "show whatever is in the filter"; a string is an uncommitted
  // draft. Committing clears the draft, so an external change to the filter
  // (a chip removed, filters cleared) reaches the field without an effect.
  const [draft, setDraft] = useState<string | null>(null)
  const text = draft ?? value ?? ''

  useEffect(() => {
    if (draft === null) return
    const trimmed = draft.trim()
    const next = trimmed.length === 0 ? null : trimmed
    const timer = setTimeout(() => {
      if (next !== (value ?? null)) setFilter(doc, { query: next })
      setDraft(null)
    }, SEARCH_DEBOUNCE_MS)
    return () => clearTimeout(timer)
  }, [doc, draft, setFilter, value])

  return (
    <input
      type="search"
      className={styles.search}
      aria-label={t('fontView.search')}
      placeholder={t('fontView.searchPlaceholder')}
      value={text}
      onChange={(event) => setDraft(event.target.value)}
    />
  )
}
