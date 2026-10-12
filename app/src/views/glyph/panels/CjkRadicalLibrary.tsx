import { useEffect, useId, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  call,
  onDocumentEvent,
  type CjkRadicalLibrary as RadicalLibraryData,
  type DocId,
  type GlyphCell,
} from '../../../api'
import { emptyFilter } from '../../../stores/documentStore'
import {
  componentEntries,
  entryChar,
  entryCodepoint,
  filterEntries,
  idsToken,
  pageGlyphNames,
  radicalEntries,
  type RadicalEntry,
} from './cjkRadicalEntries'
import panelStyles from './CjkConstructionPanel.module.css'
import sharedStyles from './panels.module.css'

const PAGE_SIZE = 48

/**
 * How long a burst of document events is collapsed before the library is
 * rebuilt, in milliseconds.
 *
 * `cjk.radicals` scans every layer of the font for component bases, so a
 * continuous gesture — a node drag with the section open — must not pay for
 * that scan once per frame. The outline request below waits the same 100 ms.
 */
const REFRESH_DELAY = 100

/** The library's two sources, in tab order. */
const TABS = ['radicals', 'components'] as const

type Tab = (typeof TABS)[number]

/** No outlines to show, for a page whose entries are all unencoded. */
const EMPTY_CELLS: ReadonlyMap<string, GlyphCell> = new Map()

/**
 * The radical library: the 214 Kangxi radicals from the pinned Unihan data with
 * the glyphs this font has for each of them, and the glyphs this font already
 * uses as components. Picking either appends it to the IDS input.
 */
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
  onPick: (token: string) => void
}) {
  const { t } = useTranslation()
  const baseId = useId()
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([])
  const [tab, setTab] = useState<Tab>('radicals')
  const [query, setQuery] = useState('')
  const [presentOnly, setPresentOnly] = useState(true)
  const [offset, setOffset] = useState(0)
  const [library, setLibrary] = useState<{
    key: string
    data: RadicalLibraryData
  } | null>(null)
  const [failed, setFailed] = useState<string | null>(null)
  // Appending components changes what the component tab lists, and the glyph
  // panel's generation prop can lag the command that caused it, so the library
  // also refetches on the document event itself.
  const [revision, setRevision] = useState(0)
  const [outlines, setOutlines] = useState<{
    key: string
    cells: ReadonlyMap<string, GlyphCell>
  } | null>(null)
  const libraryKey = JSON.stringify([doc, generation, revision])

  useEffect(
    () =>
      onDocumentEvent((event) => {
        if (event.doc === doc) setRevision((value) => value + 1)
      }),
    [doc],
  )

  useEffect(() => {
    let cancelled = false
    const timer = setTimeout(() => {
      void call('cjk.radicals', { doc })
        .then((data) => {
          if (!cancelled) {
            setLibrary({ key: libraryKey, data })
            setFailed(null)
          }
        })
        .catch(() => {
          if (!cancelled) setFailed(libraryKey)
        })
    }, REFRESH_DELAY)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [doc, libraryKey])

  const data = library?.key === libraryKey ? library.data : null
  const entries = useMemo(
    () =>
      data === null
        ? []
        : tab === 'radicals'
          ? radicalEntries(data)
          : componentEntries(data),
    [data, tab],
  )
  const matches = useMemo(
    () => filterEntries(entries, query, tab === 'radicals' && presentOnly),
    [entries, query, presentOnly, tab],
  )
  const start = Math.min(offset, Math.max(0, matches.length - 1))
  const page = useMemo(
    () => matches.slice(start, start + PAGE_SIZE),
    [matches, start],
  )

  // One outline request per page, by exact glyph name, so the preview is the
  // font's own drawing rather than a system fallback wherever one exists.
  const names = useMemo(() => pageGlyphNames(page), [page])
  const outlineKey = JSON.stringify([doc, master, generation, names])
  useEffect(() => {
    if (names.length === 0) return
    let cancelled = false
    const timer = setTimeout(() => {
      void call('glyphs.list', {
        doc,
        master,
        filter: { ...emptyFilter(), glyph_names: [...names] },
        sort: 'Unicode',
        offset: 0,
        limit: PAGE_SIZE,
        include_paths: true,
      })
        .then((result) => {
          if (cancelled) return
          setOutlines({
            key: outlineKey,
            cells: new Map(result.items.map((cell) => [cell.name, cell])),
          })
        })
        .catch(() => {
          if (!cancelled) setOutlines({ key: outlineKey, cells: new Map() })
        })
    }, REFRESH_DELAY)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [doc, master, generation, names, outlineKey])
  const cells =
    names.length === 0
      ? EMPTY_CELLS
      : outlines?.key === outlineKey
        ? outlines.cells
        : null

  const describe = (entry: RadicalEntry): string => {
    const parts: string[] = []
    if (entry.radical !== null)
      parts.push(t('cjkConstruction.radicalLabel', { number: entry.radical }))
    if (entry.strokes !== null)
      parts.push(t('cjkConstruction.strokesLabel', { count: entry.strokes }))
    if (entry.unicode !== null) parts.push(entryCodepoint(entry))
    if (entry.glyph !== null) parts.push(entry.glyph)
    parts.push(
      entry.radical === null
        ? t('cjkConstruction.usesLabel', { count: entry.count })
        : t('cjkConstruction.membersLabel', { count: entry.count }),
    )
    return parts.join(' · ')
  }

  // One tab stop for the whole strip, arrow keys between the tabs and a panel
  // each tab names: the pattern assistive technology expects of `role="tab"`.
  const show = (value: Tab) => {
    setTab(value)
    setOffset(0)
  }
  const onTabKey = (event: React.KeyboardEvent, index: number) => {
    const step =
      event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0
    const next =
      step === 0
        ? event.key === 'Home'
          ? 0
          : event.key === 'End'
            ? TABS.length - 1
            : -1
        : (index + step + TABS.length) % TABS.length
    if (next < 0) return
    event.preventDefault()
    show(TABS[next]!)
    tabRefs.current[next]?.focus()
  }

  return (
    <div className={panelStyles.library}>
      <div
        className={sharedStyles.actions}
        role="tablist"
        aria-label={t('cjkConstruction.library')}
      >
        {TABS.map((value, index) => (
          <button
            key={value}
            type="button"
            role="tab"
            id={`${baseId}-tab-${value}`}
            aria-selected={tab === value}
            aria-controls={`${baseId}-panel`}
            tabIndex={tab === value ? 0 : -1}
            ref={(element) => {
              tabRefs.current[index] = element
            }}
            className={sharedStyles.actionButton}
            onKeyDown={(event) => onTabKey(event, index)}
            onClick={() => show(value)}
          >
            {t(`cjkConstruction.tabs.${value}`)}
          </button>
        ))}
      </div>
      <div
        role="tabpanel"
        id={`${baseId}-panel`}
        aria-labelledby={`${baseId}-tab-${tab}`}
        className={panelStyles.library}
      >
        <p className={panelStyles.hint}>
          {t(
            tab === 'radicals'
              ? 'cjkConstruction.libraryHint'
              : 'cjkConstruction.componentsHint',
          )}
        </p>
        <input
          className={sharedStyles.textInput}
          aria-label={t('cjkConstruction.search')}
          value={query}
          onChange={(event) => {
            setQuery(event.currentTarget.value)
            setOffset(0)
          }}
        />
        {tab === 'radicals' && (
          <label>
            <input
              type="checkbox"
              checked={presentOnly}
              onChange={(event) => {
                setPresentOnly(event.currentTarget.checked)
                setOffset(0)
              }}
            />
            {t('cjkConstruction.presentOnly')}
          </label>
        )}
        {failed === libraryKey ? (
          <p role="alert">{t('cjkConstruction.libraryError')}</p>
        ) : data === null ? (
          <p role="status">{t('cjkConstruction.loading')}</p>
        ) : (
          <>
            <div className={panelStyles.radicals}>
              {page.map((entry) => {
                const cell = entry.glyph ? cells?.get(entry.glyph) : undefined
                const character = entryChar(entry)
                return (
                  <button
                    type="button"
                    key={entry.key}
                    className={panelStyles.radical}
                    title={describe(entry)}
                    aria-label={t('cjkConstruction.pick', {
                      name: describe(entry),
                    })}
                    disabled={entry.glyph === glyph}
                    onClick={() => onPick(idsToken(entry))}
                  >
                    {cell?.path && cell.bbox ? (
                      <svg
                        viewBox={`${cell.bbox[0]} ${-cell.bbox[3]} ${Math.max(1, cell.bbox[2] - cell.bbox[0])} ${Math.max(1, cell.bbox[3] - cell.bbox[1])}`}
                        aria-hidden="true"
                      >
                        <path d={cell.path} transform="scale(1 -1)" />
                      </svg>
                    ) : (
                      <span>{character ?? '□'}</span>
                    )}
                    <small>
                      {entry.glyph ??
                        (entry.radical === null ? '' : `#${entry.radical}`)}
                    </small>
                  </button>
                )
              })}
            </div>
            {matches.length === 0 && (
              <p className={panelStyles.hint}>
                {t(
                  tab === 'radicals'
                    ? 'cjkConstruction.noRadicals'
                    : 'cjkConstruction.noComponentBases',
                )}
              </p>
            )}
            <p className={panelStyles.hint}>
              {t('cjkConstruction.librarySource', {
                version: data.unihan_version,
              })}
            </p>
            {matches.length > PAGE_SIZE && (
              <div className={sharedStyles.actions}>
                <button
                  className={sharedStyles.actionButton}
                  type="button"
                  disabled={start === 0}
                  onClick={() => setOffset(Math.max(0, start - PAGE_SIZE))}
                >
                  {t('cjkConstruction.previous')}
                </button>
                <span>
                  {t('cjkConstruction.page', {
                    first: start + 1,
                    last: start + page.length,
                    total: matches.length,
                  })}
                </span>
                <button
                  className={sharedStyles.actionButton}
                  type="button"
                  disabled={start + PAGE_SIZE >= matches.length}
                  onClick={() => setOffset(start + PAGE_SIZE)}
                >
                  {t('cjkConstruction.next')}
                </button>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  )
}
