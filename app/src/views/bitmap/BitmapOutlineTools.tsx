import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type BitmapStrikeInfo,
  type BitmapTraceMode,
  type DocId,
} from '../../api'
import { withSettledDocument } from '../../editor/documentPersistence'
import { openGlyphTab } from '../../shell/tabActions'
import { emptyFilter } from '../../stores/documentStore'
import { reportError } from '../../stores/uiStore'
import styles from './BitmapView.module.css'

export function BitmapOutlineTools({
  doc,
  strike,
  glyph,
  master,
}: {
  doc: DocId
  strike: BitmapStrikeInfo
  glyph: string
  master: string
}) {
  const { t } = useTranslation()
  const [threshold, setThreshold] = useState(128)
  const [mode, setMode] = useState<BitmapTraceMode>('PixelEdges')
  const [join, setJoin] = useState(false)
  const [pixel, setPixel] = useState('pixel')
  const [all, setAll] = useState(false)
  const [traceAll, setTraceAll] = useState(false)
  const [busy, setBusy] = useState(false)
  const valid =
    Number.isInteger(threshold) && threshold >= 1 && threshold <= 255
  const run = (components: boolean) => {
    if (busy || !valid) return
    setBusy(true)
    void withSettledDocument(doc, async () => {
      const state = await call('bitmap.state', { doc })
      if (components)
        await call('bitmap.pixel_components', {
          doc,
          expected_generation: state.generation,
          ppem: strike.ppem,
          depth: strike.depth,
          master,
          glyphs: all ? [] : [glyph],
          threshold,
          pixel_glyph: pixel,
        })
      else
        await call('bitmap.trace', {
          doc,
          expected_generation: state.generation,
          ppem: strike.ppem,
          depth: strike.depth,
          master,
          glyph,
          // One undo step either way: the single glyph, or every image in this
          // strike when the batch switch is on.
          glyphs: [],
          all_images: traceAll,
          threshold,
          mode,
          join_diagonals: join,
        })
    })
      .catch(reportError)
      .finally(() => setBusy(false))
  }
  const editPixelGlyph = () => {
    if (busy) return
    setBusy(true)
    void call('glyphs.list', {
      doc,
      master: null,
      filter: { ...emptyFilter(), query: pixel.trim() },
      sort: 'Order',
      offset: 0,
      limit: 20,
      include_paths: false,
    })
      .then(async (page) => {
        const found = page.items.find((item) => item.name === pixel.trim())
        if (!found) throw new Error(t('bitmapParity.pixelMissing'))
        await openGlyphTab(doc, found.gid)
      })
      .catch(reportError)
      .finally(() => setBusy(false))
  }
  return (
    <div>
      <div className={styles.toolbar}>
        <label>
          {t('bitmap.threshold')}
          <input
            type="number"
            min={1}
            max={255}
            value={threshold}
            onChange={(event) => setThreshold(Number(event.target.value))}
          />
        </label>
        <label>
          {t('bitmapParity.cornerMode')}
          <select
            value={mode}
            onChange={(event) => setMode(event.target.value as BitmapTraceMode)}
          >
            {(
              [
                'PixelEdges',
                'ChamferedCorners',
                'RoundedCorners',
                'DiagonalSmoothing',
              ] as const
            ).map((value) => (
              <option key={value} value={value}>
                {t(`bitmapParity.modes.${value}`)}
              </option>
            ))}
          </select>
        </label>
        <label>
          <input
            type="checkbox"
            checked={join}
            onChange={(event) => setJoin(event.target.checked)}
          />
          {t('bitmapParity.joinDiagonals')}
        </label>
        <label>
          <input
            type="checkbox"
            checked={traceAll}
            onChange={(event) => setTraceAll(event.target.checked)}
          />
          {t('bitmapParity.traceAll', { count: strike.glyph_count })}
        </label>
        <button disabled={busy || !valid} onClick={() => run(false)}>
          {t('bitmap.trace')}
        </button>
      </div>
      <details>
        <summary>{t('bitmapParity.pixelComponents')}</summary>
        <div className={styles.toolbar}>
          <label>
            {t('bitmapParity.pixelGlyph')}
            <input
              value={pixel}
              maxLength={255}
              onChange={(event) => setPixel(event.target.value)}
            />
          </label>
          <label>
            <input
              type="checkbox"
              checked={all}
              onChange={(event) => setAll(event.target.checked)}
            />
            {t('bitmapParity.allImages')}
          </label>
          <button
            disabled={busy || !valid || !pixel.trim()}
            onClick={() => run(true)}
          >
            {t('bitmapParity.buildComponents')}
          </button>
          <button
            type="button"
            disabled={busy || !pixel.trim()}
            onClick={editPixelGlyph}
          >
            {t('bitmapParity.editPixelGlyph')}
          </button>
        </div>
        <p className={styles.hint}>{t('bitmapParity.componentHint')}</p>
      </details>
    </div>
  )
}
