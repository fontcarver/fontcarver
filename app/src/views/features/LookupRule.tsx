import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call, type DocId, type LayoutRuleInfo, type MasterId } from '../../api'

import { LookupGlyph } from './LookupGlyph'
import styles from './LookupInspector.module.css'
import { useLayoutQuery } from './useLayoutQuery'

interface LookupRuleProps {
  doc: DocId
  master: MasterId | null
  generation: number
  table: string
  lookup: number
  index: number
  rule: LayoutRuleInfo
}

const GLYPHS_PER_PAGE = 8

/** A bounded source rendering and independently paged referenced glyphs. */
export function LookupRule({
  doc,
  master,
  generation,
  table,
  lookup,
  index,
  rule,
}: LookupRuleProps) {
  const { t } = useTranslation()
  const [offset, setOffset] = useState(0)
  const [refresh, setRefresh] = useState(0)
  const identity = JSON.stringify([
    doc,
    master,
    generation,
    table,
    lookup,
    index,
    offset,
    refresh,
  ])
  const answer = useLayoutQuery(identity, () =>
    call('layout.rule_glyphs', {
      doc,
      table,
      lookup,
      rule: index,
      generation,
      offset,
      limit: GLYPHS_PER_PAGE,
    }),
  )
  const page = answer?.value
  return (
    <li className={styles.rule}>
      <code className={styles.ruleText}>{rule.text}</code>
      {rule.truncated && (
        <p className={styles.hint}>{t('layoutInspector.ruleTruncated')}</p>
      )}
      <div
        className={styles.glyphs}
        aria-label={t('layoutInspector.referencedGlyphs')}
      >
        {page?.items.map((glyph, position) =>
          master ? (
            <LookupGlyph
              key={`${glyph.name}:${position}`}
              doc={doc}
              master={master}
              generation={generation}
              glyph={glyph}
            />
          ) : (
            <code key={`${glyph.name}:${position}`}>{glyph.name}</code>
          ),
        )}
      </div>
      {answer?.failed && (
        <button
          type="button"
          className={styles.button}
          onClick={() => setRefresh((value) => value + 1)}
        >
          {t('layoutInspector.retryGlyphs')}
        </button>
      )}
      {page && (offset > 0 || page.next_offset !== null) && (
        <div className={styles.pager}>
          <button
            type="button"
            className={styles.button}
            disabled={offset === 0}
            onClick={() => setOffset(Math.max(0, offset - GLYPHS_PER_PAGE))}
          >
            {t('layoutInspector.previousGlyphs')}
          </button>
          <span className={styles.hint}>
            {t('layoutInspector.page', {
              first: offset + 1,
              last: offset + page.items.length,
              total: page.total,
            })}
          </span>
          <button
            type="button"
            className={styles.button}
            disabled={page.next_offset === null}
            onClick={() => {
              if (page.next_offset !== null) setOffset(page.next_offset)
            }}
          >
            {t('layoutInspector.nextGlyphs')}
          </button>
        </div>
      )}
    </li>
  )
}
