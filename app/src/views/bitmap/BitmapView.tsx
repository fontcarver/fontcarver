import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  onDocumentEvent,
  type BitmapCommand,
  type BitmapHinting,
  type BitmapState,
  type DocSummary,
  type GlyphCell,
} from '../../api'
import { withSettledDocument } from '../../editor/documentPersistence'
import { emptyFilter, useDocumentStore } from '../../stores/documentStore'
import { reportError } from '../../stores/uiStore'
import { BitmapGridPreset } from './BitmapGridPreset'
import { BitmapSheet } from './BitmapSheet'
import { BitmapTransfer } from './BitmapTransfer'
import { BitmapOutlineTools } from './BitmapOutlineTools'
import { BitmapReconstruction } from './BitmapReconstruction'
import { StrikeProperties } from './StrikeProperties'
import { BitmapEditor } from './BitmapEditor'
import { BitmapWaterfall } from './BitmapWaterfall'
import styles from './BitmapView.module.css'
export function BitmapView({ summary }: { summary: DocSummary }) {
  const { t } = useTranslation(),
    doc = summary.doc
  const [state, setState] = useState<BitmapState>({
      generation: 0,
      strikes: [],
    }),
    [strikeKey, setStrikeKey] = useState(''),
    [glyph, setGlyph] = useState(''),
    [query, setQuery] = useState(''),
    [offset, setOffset] = useState(0),
    [items, setItems] = useState<GlyphCell[]>([]),
    [total, setTotal] = useState(0)
  const [ppem, setPpem] = useState(16),
    [strikeMetrics, setStrikeMetrics] = useState<{
      ascent: number
      descent: number
    } | null>(null),
    [depth, setDepth] = useState(1),
    [busy, setBusy] = useState(false),
    [master, setMaster] = useState(summary.masters[0]?.id ?? ''),
    [all, setAll] = useState(false)
  const refresh = useCallback(async () => {
    const next = await call('bitmap.state', { doc })
    setState((old) => (next.generation >= old.generation ? next : old))
    setStrikeKey((old) =>
      next.strikes.some((s) => `${s.ppem}:${s.depth}` === old)
        ? old
        : next.strikes[0]
          ? `${next.strikes[0].ppem}:${next.strikes[0].depth}`
          : '',
    )
  }, [doc])
  useEffect(() => {
    void Promise.resolve().then(refresh).catch(reportError)
    return onDocumentEvent((event) => {
      if (event.doc === doc) void refresh().catch(reportError)
    })
  }, [doc, refresh])
  useEffect(() => {
    let current = true
    const timer = setTimeout(() => {
      void call('glyphs.list', {
        doc,
        master: null,
        filter: { ...emptyFilter(), query: query || null },
        sort: 'Order',
        offset,
        limit: 80,
        include_paths: false,
      })
        .then((page) => {
          if (!current) return
          setItems(page.items)
          setTotal(page.total)
          setGlyph(
            (old) =>
              old ||
              page.items.find(
                (g) => g.gid === useDocumentStore.getState().byDoc[doc]?.focus,
              )?.name ||
              page.items[0]?.name ||
              '',
          )
        })
        .catch(reportError)
    }, 100)
    return () => {
      current = false
      clearTimeout(timer)
    }
  }, [doc, query, offset, summary.glyph_count])
  const [hinting, setHinting] = useState<BitmapHinting>('None')
  const [rasterThreshold, setRasterThreshold] = useState(128)
  const strike = state.strikes.find((s) => `${s.ppem}:${s.depth}` === strikeKey)
  const selectedCell = items.find((item) => item.name === glyph)
  const operation = (run: () => Promise<unknown>) => {
    if (busy) return
    setBusy(true)
    void withSettledDocument(doc, run)
      .then(refresh)
      .catch(reportError)
      .finally(() => setBusy(false))
  }
  const apply = (command: BitmapCommand) =>
    operation(async () => {
      const current = await call('bitmap.state', { doc })
      return call('bitmap.apply', {
        doc,
        expected_generation: current.generation,
        command,
        label: 'Edit bitmap strike',
      })
    })
  const select = (action: () => void) =>
    operation(async () => {
      action()
    })
  return (
    <div className={styles.view}>
      <header className={styles.header}>
        <div>
          <h1>{t('bitmap.title')}</h1>
          <p>
            {summary.family_name} ·{' '}
            {t('bitmap.strikeCount', { count: state.strikes.length })}
          </p>
        </div>
        <label>
          {t('bitmap.strike')}
          <select
            disabled={busy}
            value={strikeKey}
            onChange={(e) => {
              const next = e.target.value
              select(() => setStrikeKey(next))
            }}
          >
            <option value="" disabled>
              {t('bitmap.noStrike')}
            </option>
            {state.strikes.map((s) => (
              <option
                key={`${s.ppem}:${s.depth}`}
                value={`${s.ppem}:${s.depth}`}
              >
                {s.ppem}px · {s.depth}bpp · {s.glyph_count}
              </option>
            ))}
          </select>
        </label>
        <button
          disabled={!strike || busy}
          onClick={() => {
            if (strike)
              apply({
                RemoveStrike: { ppem: strike.ppem, depth: strike.depth },
              })
          }}
        >
          {t('bitmap.removeStrike')}
        </button>
      </header>
      <div className={styles.layout}>
        <aside className={styles.sidebar}>
          <label>
            {t('bitmap.search')}
            <input
              value={query}
              onChange={(e) => {
                setQuery(e.target.value)
                setOffset(0)
              }}
            />
          </label>
          <div className={styles.glyphList}>
            {items.map((item) => (
              <button
                key={item.gid}
                aria-pressed={glyph === item.name}
                disabled={busy}
                onClick={() => select(() => setGlyph(item.name))}
              >
                <span>
                  {item.unicodes[0] !== undefined
                    ? String.fromCodePoint(item.unicodes[0])
                    : '·'}
                </span>
                <span>
                  {item.name}
                  <small>
                    {item.unicodes
                      .map((u) => `U+${u.toString(16).toUpperCase()}`)
                      .join(' ')}
                  </small>
                </span>
              </button>
            ))}
          </div>
          <div className={styles.toolbar}>
            <button
              disabled={offset === 0}
              onClick={() => setOffset(Math.max(0, offset - 80))}
            >
              {t('bitmap.previous')}
            </button>
            <span>
              {Math.min(offset + 1, total)}–{Math.min(offset + 80, total)} /{' '}
              {total}
            </span>
            <button
              disabled={offset + 80 >= total}
              onClick={() => setOffset(offset + 80)}
            >
              {t('bitmap.next')}
            </button>
          </div>
        </aside>
        <div className={styles.main}>
          <BitmapReconstruction
            key={`${doc}:${master}`}
            doc={doc}
            master={master}
          />
          <div className={styles.creation}>
            <BitmapGridPreset
              doc={doc}
              master={master}
              value={ppem}
              onSelect={(choice) => {
                setPpem(choice.ppem)
                setStrikeMetrics({
                  ascent: choice.ascent,
                  descent: choice.descent,
                })
              }}
              disabled={busy}
            />
            <label>
              {t('bitmap.size')}
              <input
                type="number"
                min={1}
                max={4096}
                value={ppem}
                onChange={(e) => {
                  setPpem(Number(e.target.value))
                  setStrikeMetrics(null)
                }}
              />
            </label>
            <label>
              {t('bitmap.depth')}
              <select
                value={depth}
                onChange={(e) => setDepth(Number(e.target.value))}
              >
                {[1, 2, 4, 8, 32].map((n) => (
                  <option key={n} value={n}>
                    {n === 32 ? 'RGBA' : `${n} bpp`}
                  </option>
                ))}
              </select>
            </label>
            <button
              disabled={
                busy ||
                !Number.isInteger(ppem) ||
                ppem < 1 ||
                ppem > 4096 ||
                state.strikes.some((s) => s.ppem === ppem && s.depth === depth)
              }
              onClick={() =>
                apply({
                  SetStrike: {
                    strike: {
                      ppem,
                      depth,
                      // A grid preset carries the metrics the kernel derived
                      // from the master; a typed size falls back to 80/20.
                      ascent: strikeMetrics?.ascent ?? Math.ceil(ppem * 0.8),
                      descent: strikeMetrics?.descent ?? Math.floor(ppem * 0.2),
                      properties: {},
                      glyphs: {},
                    },
                  },
                })
              }
            >
              {t('bitmap.create')}
            </button>
            <label>
              {t('bitmap.master')}
              <select
                value={master}
                onChange={(e) => setMaster(e.target.value)}
              >
                {summary.masters.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.name}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <input
                type="checkbox"
                checked={all}
                onChange={(e) => setAll(e.target.checked)}
              />
              {t('bitmap.allGlyphs')}
            </label>
            <button
              disabled={
                busy ||
                depth === 32 ||
                (!glyph && !all) ||
                !Number.isInteger(ppem) ||
                ppem < 1 ||
                ppem > 4096
              }
              onClick={() =>
                operation(async () => {
                  const current = await call('bitmap.state', { doc })
                  await call('bitmap.rasterize', {
                    doc,
                    master,
                    expected_generation: current.generation,
                    ppem,
                    depth,
                    glyphs: all ? [] : [glyph],
                    hinting,
                    threshold: rasterThreshold,
                  })
                  setStrikeKey(`${ppem}:${depth}`)
                })
              }
            >
              {t('bitmap.rasterize')}
            </button>
          </div>
          <div className={styles.toolbar}>
            <label>
              {t('bitmap.hinting')}
              <select
                value={hinting}
                onChange={(e) => setHinting(e.target.value as BitmapHinting)}
              >
                {(['None', 'Embedded', 'Auto'] as const).map((mode) => (
                  <option key={mode} value={mode}>
                    {t(`bitmap.hint.${mode}`)}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t('bitmap.rasterThreshold')}
              <input
                type="number"
                min={1}
                max={255}
                value={rasterThreshold}
                onChange={(e) => setRasterThreshold(Number(e.target.value))}
              />
            </label>
          </div>
          <p className={styles.hint}>{t('bitmap.rasterHint')}</p>
          {strike && glyph ? (
            <>
              <h2>
                {glyph}{' '}
                <span>
                  {strike.ppem}px · {strike.depth}bpp
                </span>
              </h2>
              <BitmapEditor
                key={`${doc}:${strikeKey}:${glyph}`}
                doc={doc}
                glyph={glyph}
                strike={strike}
                upm={summary.units_per_em}
                master={master}
                gid={selectedCell?.gid}
              />
              <BitmapOutlineTools
                doc={doc}
                strike={strike}
                glyph={glyph}
                master={master}
              />
            </>
          ) : (
            <p className={styles.empty}>{t('bitmap.empty')}</p>
          )}
          {strike && (
            <>
              <BitmapTransfer
                key={`transfer:${strikeKey}`}
                doc={doc}
                strike={strike}
                glyph={glyph}
              />
              <BitmapSheet
                key={`sheet:${doc}:${strikeKey}`}
                doc={doc}
                strike={strike}
              />
              <StrikeProperties doc={doc} strike={strike} />
            </>
          )}
          <BitmapWaterfall
            doc={doc}
            revision={state.generation}
            glyph={glyph || undefined}
          />
        </div>
      </div>
    </div>
  )
}
