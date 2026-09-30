import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call, type DocId, type GlyphListResult } from '../../../api'
import { emptyFilter } from '../../../stores/documentStore'
import { radicalCodepoints } from './cjkConstruction'
import panelStyles from './CjkConstructionPanel.module.css'
import sharedStyles from './panels.module.css'

const PAGE_SIZE = 48
const RANGES = radicalCodepoints().map((point): [number, number] => [
  point,
  point,
])

/** A paged library of radical outlines already present in this document. */
export function CjkRadicalLibrary({
  doc,
  master,
  glyph,
  generation,
  onPick,
}: {
  doc: DocId
  master: string | null
  glyph: string
  generation: number
  onPick: (name: string) => void
}) {
  const { t } = useTranslation()
  const [query, setQuery] = useState('')
  const [offset, setOffset] = useState(0)
  const [result, setResult] = useState<{
    key: string
    page: GlyphListResult
  } | null>(null)
  const [failed, setFailed] = useState<string | null>(null)
  const key = JSON.stringify([doc, master, generation, query, offset])

  useEffect(() => {
    let cancelled = false
    const timer = setTimeout(() => {
      void call('glyphs.list', {
        doc,
        master,
        filter: {
          ...emptyFilter(),
          query: query.trim() || null,
          unicode_ranges: RANGES,
        },
        sort: 'Unicode',
        offset,
        limit: PAGE_SIZE,
        include_paths: true,
      })
        .then((page) => {
          if (!cancelled) {
            setResult({ key, page })
            setFailed(null)
          }
        })
        .catch(() => {
          if (!cancelled) setFailed(key)
        })
    }, 100)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [doc, master, generation, query, offset, key])

  const page = result?.key === key ? result.page : null
  return (
    <div className={panelStyles.library}>
      <p className={panelStyles.hint}>{t('cjkConstruction.libraryHint')}</p>
      <input
        className={sharedStyles.textInput}
        aria-label={t('cjkConstruction.search')}
        value={query}
        onChange={(event) => {
          setQuery(event.currentTarget.value)
          setOffset(0)
        }}
      />
      {failed === key ? (
        <p role="alert">{t('cjkConstruction.libraryError')}</p>
      ) : !page ? (
        <p role="status">{t('cjkConstruction.loading')}</p>
      ) : (
        <>
          <div className={panelStyles.radicals}>
            {page.items.map((cell) => (
              <button
                type="button"
                key={cell.gid}
                className={panelStyles.radical}
                title={cell.name}
                aria-label={t('cjkConstruction.pick', { name: cell.name })}
                disabled={cell.name === glyph || cell.is_empty}
                onClick={() => onPick(cell.name)}
              >
                {cell.path && cell.bbox ? (
                  <svg
                    viewBox={`${cell.bbox[0]} ${-cell.bbox[3]} ${Math.max(1, cell.bbox[2] - cell.bbox[0])} ${Math.max(1, cell.bbox[3] - cell.bbox[1])}`}
                    aria-hidden="true"
                  >
                    <path d={cell.path} transform="scale(1 -1)" />
                  </svg>
                ) : (
                  <span>
                    {String.fromCodePoint(cell.unicodes[0] ?? 0x25a1)}
                  </span>
                )}
                <small>{cell.name}</small>
              </button>
            ))}
          </div>
          {page.total === 0 && (
            <p className={panelStyles.hint}>
              {t('cjkConstruction.noRadicals')}
            </p>
          )}
          {page.total > PAGE_SIZE && (
            <div className={sharedStyles.actions}>
              <button
                className={sharedStyles.actionButton}
                type="button"
                disabled={offset === 0}
                onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
              >
                {t('cjkConstruction.previous')}
              </button>
              <span>
                {t('cjkConstruction.page', {
                  first: offset + 1,
                  last: offset + page.items.length,
                  total: page.total,
                })}
              </span>
              <button
                className={sharedStyles.actionButton}
                type="button"
                disabled={offset + PAGE_SIZE >= page.total}
                onClick={() => setOffset(offset + PAGE_SIZE)}
              >
                {t('cjkConstruction.next')}
              </button>
            </div>
          )}
        </>
      )}
    </div>
  )
}
