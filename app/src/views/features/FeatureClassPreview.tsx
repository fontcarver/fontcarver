import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import {
  call,
  type DocId,
  type Features,
  type GlyphId,
  type MasterId,
} from '../../api'

import styles from './FeatureClassPreview.module.css'
import { LookupGlyph } from './LookupGlyph'
import { resolveFeatureClassMembers } from './featureClassMembers'
import { useLayoutQuery } from './useLayoutQuery'

/** Source-based, read-only class preview in the feature editor. */
export interface FeatureClassPreviewProps {
  doc: DocId
  master: MasterId | null
  generation: number
  source: Features
  name: string
  onClose: () => void
}

const PREVIEW_LIMIT = 64
const SEARCH_CONCURRENCY = 8

async function resolveGlyphs(doc: DocId, names: string[]) {
  const glyphs: { name: string; gid: GlyphId | null }[] = names.map((name) => ({
    name,
    gid: null,
  }))
  let next = 0
  await Promise.all(
    Array.from(
      { length: Math.min(SEARCH_CONCURRENCY, names.length) },
      async () => {
        while (next < names.length) {
          const index = next++
          const name = names[index]
          try {
            const matches = await call('glyphs.search', {
              doc,
              query: name,
              limit: 1,
            })
            glyphs[index].gid =
              matches.find((match) => match.name === name)?.gid ?? null
          } catch {
            // Preserve source names even when a lookup fails.
          }
        }
      },
    ),
  )
  return glyphs
}

/** Shows resolved member names and actual glyph outlines for the current master. */
export function FeatureClassPreview({
  doc,
  master,
  generation,
  source,
  name,
  onClose,
}: FeatureClassPreviewProps) {
  const { t } = useTranslation()
  const resolved = useMemo(
    () => resolveFeatureClassMembers(source, name),
    [source, name],
  )
  const visible = resolved.members.slice(0, PREVIEW_LIMIT)
  const identity = JSON.stringify([
    doc,
    master,
    generation,
    name,
    source.classes,
  ])
  const answer = useLayoutQuery(identity, async () =>
    master === null ? [] : resolveGlyphs(doc, visible),
  )
  const displayName = name.replace(/^@/, '')
  const incomplete = resolved.issues.length > 0

  return (
    <section
      className={styles.preview}
      aria-label={t('features.classPreview.title', { name: displayName })}
      onKeyDown={(event) => {
        if (event.key === 'Escape') {
          event.stopPropagation()
          onClose()
        }
      }}
    >
      <div className={styles.header}>
        <h3>{t('features.classPreview.title', { name: displayName })}</h3>
        <span className={styles.hint}>
          {t('features.classPreview.count', { count: resolved.members.length })}
        </span>
        <button type="button" className={styles.close} onClick={onClose}>
          {t('features.classPreview.close')}
        </button>
      </div>
      {incomplete && (
        <p className={styles.warning}>
          {t('features.classPreview.incomplete')}
        </p>
      )}
      {resolved.issues.map((issue) => (
        <p key={`${issue.kind}:${issue.name}`} className={styles.warning}>
          {t(`features.classPreview.${issue.kind}`, { name: issue.name })}
        </p>
      ))}
      {resolved.members.length > PREVIEW_LIMIT && (
        <p className={styles.hint}>
          {t('features.classPreview.showing', { count: PREVIEW_LIMIT })}
        </p>
      )}
      {master === null && visible.length > 0 && (
        <p className={styles.hint}>{t('features.classPreview.noMaster')}</p>
      )}
      {master !== null && !answer && visible.length > 0 && (
        <p className={styles.hint} role="status">
          {t('features.classPreview.loading')}
        </p>
      )}
      {answer?.value?.some((glyph) => glyph.gid === null) && (
        <p className={styles.warning}>
          {t('features.classPreview.unresolved')}
        </p>
      )}
      {!visible.length && !incomplete && (
        <p className={styles.hint}>{t('features.classPreview.empty')}</p>
      )}
      {visible.length > 0 && (
        <ul
          className={styles.members}
          aria-label={t('features.classPreview.members', { name: displayName })}
        >
          {visible.map((glyphName, index) => (
            <li key={glyphName}>
              {master !== null && answer?.value?.[index] ? (
                <LookupGlyph
                  doc={doc}
                  master={master}
                  generation={generation}
                  glyph={answer.value[index]}
                />
              ) : (
                <code className={styles.name}>{glyphName}</code>
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
