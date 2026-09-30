import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type BitmapStrikeInfo, type DocId } from '../../api'
import { withSettledDocument } from '../../editor/documentPersistence'
import { reportError } from '../../stores/uiStore'
import styles from './BitmapView.module.css'

export function BitmapTransfer({
  doc,
  strike,
  glyph,
}: {
  doc: DocId
  strike: BitmapStrikeInfo
  glyph: string
}) {
  const { t } = useTranslation()
  const [ppem, setPpem] = useState(Math.min(4096, strike.ppem * 2))
  const [depth, setDepth] = useState(strike.depth)
  const [scale, setScale] = useState(true)
  const [all, setAll] = useState(false)
  const [busy, setBusy] = useState(false)
  const valid =
    Number.isInteger(ppem) &&
    ppem >= 1 &&
    ppem <= 4096 &&
    (ppem !== strike.ppem || depth !== strike.depth) &&
    (all || Boolean(glyph))
  return (
    <details>
      <summary>{t('bitmapParity.transfer')}</summary>
      <form
        className={styles.toolbar}
        onSubmit={(event) => {
          event.preventDefault()
          if (!valid || busy) return
          setBusy(true)
          void withSettledDocument(doc, async () => {
            const state = await call('bitmap.state', { doc })
            await call('bitmap.transfer', {
              doc,
              expected_generation: state.generation,
              source_ppem: strike.ppem,
              source_depth: strike.depth,
              target_ppem: ppem,
              target_depth: depth,
              glyphs: all ? [] : [glyph],
              scale,
            })
          })
            .catch(reportError)
            .finally(() => setBusy(false))
        }}
      >
        <label>
          {t('bitmapParity.targetSize')}
          <input
            type="number"
            min={1}
            max={4096}
            value={ppem}
            onChange={(event) => setPpem(Number(event.target.value))}
          />
        </label>
        <label>
          {t('bitmapParity.targetDepth')}
          <select
            value={depth}
            onChange={(event) => setDepth(Number(event.target.value))}
          >
            {[1, 2, 4, 8, 32].map((value) => (
              <option key={value} value={value}>
                {value === 32 ? 'RGBA' : `${value} bpp`}
              </option>
            ))}
          </select>
        </label>
        <label>
          <input
            type="checkbox"
            checked={scale}
            onChange={(event) => setScale(event.target.checked)}
          />
          {t('bitmapParity.scaleToSize')}
        </label>
        <label>
          <input
            type="checkbox"
            checked={all}
            onChange={(event) => setAll(event.target.checked)}
          />
          {t('bitmapParity.allImages')}
        </label>
        <button disabled={!valid || busy}>
          {t('bitmapParity.copyStrike')}
        </button>
      </form>
      <p className={styles.hint}>{t('bitmapParity.transferHint')}</p>
    </details>
  )
}
