import { useEffect, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocId } from '../../api'
import { useDocumentStore } from '../../stores/documentStore'

import styles from './glyphSearch.module.css'
import { selectQueryError, useQueryErrors } from './queryErrors'

/** Delay between the last keystroke and the filtered request, in milliseconds. */
export const SEARCH_DEBOUNCE_MS = 150

/** Glyph fields available to the query controls. */
export type SearchQueryScope = 'All' | 'Name' | 'Unicode' | 'Note'

export interface GlyphSearchProps {
  doc: DocId
  /** The query currently in the filter, so external changes reach the field. */
  value: string | null
  queryScope?: SearchQueryScope
  caseSensitive?: boolean
  regex?: boolean
  onQueryScopeChange?: (scope: SearchQueryScope) => void
  onCaseSensitiveChange?: (enabled: boolean) => void
  onRegexChange?: (enabled: boolean) => void
}

/**
 * Debounced glyph query with scope and matching controls. Regular expressions
 * are validated by the kernel, preserving its bounded matching semantics; a
 * rejected pattern is reported under the field rather than thrown at the user as
 * a toast, and the document is left exactly as it was.
 */
export function GlyphSearch({
  doc,
  value,
  queryScope,
  caseSensitive,
  regex,
  onQueryScopeChange,
  onCaseSensitiveChange,
  onRegexChange,
}: GlyphSearchProps) {
  const { t } = useTranslation()
  const errorId = useId()
  const setFilter = useDocumentStore((state) => state.setFilter)
  const queryError = useQueryErrors(selectQueryError(doc, 'search'))
  const storedScope = useDocumentStore(
    (state) => state.byDoc[doc]?.filter.query_scope,
  )
  const storedCase = useDocumentStore(
    (state) => state.byDoc[doc]?.filter.case_sensitive,
  )
  const storedRegex = useDocumentStore(
    (state) => state.byDoc[doc]?.filter.regex,
  )
  const scope = queryScope ?? storedScope ?? 'All'
  const matchCase = caseSensitive ?? storedCase ?? false
  const useRegex = regex ?? storedRegex ?? false
  // `null` means "show whatever is in the filter"; a string is an uncommitted
  // draft. Committing clears the draft, so an external change to the filter
  // (a chip removed, filters cleared) reaches the field without an effect.
  const [draft, setDraft] = useState<string | null>(null)
  const text = draft ?? value ?? ''

  useEffect(() => {
    if (draft === null) return
    const query = useRegex ? draft : draft.trim()
    const next = query.length === 0 ? null : query
    const timer = setTimeout(() => {
      if (next !== (value ?? null)) setFilter(doc, { query: next })
      setDraft(null)
    }, SEARCH_DEBOUNCE_MS)
    return () => clearTimeout(timer)
  }, [doc, draft, setFilter, useRegex, value])

  return (
    <div className={styles.controls}>
      <input
        type="search"
        className={styles.search}
        aria-label={t('fontView.search')}
        placeholder={t(`glyphSearch.placeholder.${scope}`)}
        aria-invalid={queryError === null ? undefined : true}
        aria-describedby={queryError === null ? undefined : errorId}
        value={text}
        onChange={(event) => setDraft(event.target.value)}
      />
      <select
        className={styles.scope}
        aria-label={t('glyphSearch.scopeLabel')}
        value={scope}
        onChange={(event) => {
          const next = event.target.value as SearchQueryScope
          if (onQueryScopeChange) onQueryScopeChange(next)
          else setFilter(doc, { query_scope: next })
        }}
      >
        {(['All', 'Name', 'Unicode', 'Note'] as const).map((option) => (
          <option key={option} value={option}>
            {t(`glyphSearch.scope.${option}`)}
          </option>
        ))}
      </select>
      <button
        type="button"
        className={styles.toggle}
        aria-label={t('glyphSearch.caseSensitive')}
        title={t('glyphSearch.caseSensitive')}
        aria-pressed={matchCase}
        onClick={() => {
          if (onCaseSensitiveChange) onCaseSensitiveChange(!matchCase)
          else setFilter(doc, { case_sensitive: !matchCase })
        }}
      >
        {t('glyphSearch.caseSymbol')}
      </button>
      <button
        type="button"
        className={styles.toggle}
        aria-label={t('glyphSearch.regex')}
        title={t('glyphSearch.regex')}
        aria-pressed={useRegex}
        onClick={() => {
          if (onRegexChange) onRegexChange(!useRegex)
          else setFilter(doc, { regex: !useRegex })
        }}
      >
        {t('glyphSearch.regexSymbol')}
      </button>
      {queryError !== null && (
        <p id={errorId} role="alert" className={styles.error}>
          {t('glyphSearch.invalidQuery', {
            // The kernel names the offending part of the pattern; a translated
            // sentence carries it rather than the raw message standing alone.
            message: queryError.replace(/\s+/gu, ' ').trim(),
          })}
        </p>
      )}
    </div>
  )
}
