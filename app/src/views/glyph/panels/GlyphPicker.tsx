import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call, type DocId, type GlyphCell } from '../../../api'
import { fitGlyph, paintGlyph } from '../../../canvas/glyphPaint'
import { pathCache } from '../../../canvas/pathCache'
import { reportError } from '../../../shell/commands'
import { useGlyphTheme } from '../glyphTheme'

import styles from './panels.module.css'

/** How many matches the popover offers before the user narrows the search. */
const LIMIT = 40

/** Thumbnail edge in CSS pixels. */
const THUMB = 24

function Thumbnail({ doc, cell }: { doc: DocId; cell: GlyphCell }) {
  const ref = useRef<HTMLCanvasElement>(null)
  const theme = useGlyphTheme()

  useEffect(() => {
    const canvas = ref.current
    if (!canvas || !cell.path) return
    const dpr = globalThis.devicePixelRatio || 1
    const device = Math.max(1, Math.round(THUMB * dpr))
    canvas.width = device
    canvas.height = device
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    ctx.clearRect(0, 0, THUMB, THUMB)
    const box = { x: 1, y: 1, width: THUMB - 2, height: THUMB - 2 }
    const em = { upem: 1000, ascender: 800, descender: -200 }
    const path = pathCache.get(doc, cell.gid, cell.generation, cell.path)
    if (path)
      paintGlyph(ctx, path, fitGlyph(box, cell.width, em), theme.outline)
  }, [cell, doc, theme.outline])

  return (
    <canvas
      ref={ref}
      className={styles.pickerThumb}
      style={{ width: THUMB, height: THUMB }}
      aria-hidden="true"
    />
  )
}

export interface GlyphPickerProps {
  doc: DocId
  /** Names already used, shown but not offered twice in a row. */
  label: string
  onPick: (name: string) => void
  onClose: () => void
}

/**
 * The search popover the Components panel adds a base glyph from (design spec
 * §7.4). It runs the same `glyphs.search` the Font view's search box uses, so
 * a name, a `U+` scalar and a `uniXXXX` alias all find the glyph.
 */
export function GlyphPicker({ doc, label, onPick, onClose }: GlyphPickerProps) {
  const { t } = useTranslation()
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<GlyphCell[]>([])

  const text = query.trim()
  // Derived rather than cleared in the effect: an empty box shows nothing
  // whatever the last search answered, without a second render to get there.
  const visible = text === '' ? [] : results

  useEffect(() => {
    if (text === '') return
    let cancelled = false
    // The kernel answers a prefix search in a few milliseconds, but the user
    // types faster than that; one short debounce keeps it to one call a word.
    const timer = setTimeout(() => {
      void call('glyphs.search', { doc, query: text, limit: LIMIT })
        .then((cells) => {
          if (!cancelled) setResults(cells)
        })
        .catch(reportError)
    }, 150)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [doc, text])

  return (
    <div
      className={styles.picker}
      role="group"
      aria-label={label}
      onKeyDown={(event) => {
        if (event.key !== 'Escape') return
        event.stopPropagation()
        onClose()
      }}
    >
      <input
        className={styles.textInput}
        value={query}
        autoFocus
        aria-label={t('panel.components.search')}
        placeholder={t('fontView.searchPlaceholder')}
        onChange={(event) => setQuery(event.currentTarget.value)}
      />
      <div className={styles.pickerResults}>
        {visible.map((cell) => (
          <button
            key={cell.gid}
            type="button"
            className={styles.pickerRow}
            onClick={() => onPick(cell.name)}
          >
            <Thumbnail doc={doc} cell={cell} />
            <span className={styles.rowName}>{cell.name}</span>
          </button>
        ))}
        {text !== '' && visible.length === 0 ? (
          <p className={styles.empty}>{t('fontView.noMatches')}</p>
        ) : null}
      </div>
    </div>
  )
}
