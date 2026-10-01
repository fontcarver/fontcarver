import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type BitmapPreviewLine, type DocId } from '../../api'
import { renderBitmap } from '../../canvas/bitmapRenderer'
import { reportError, useUiStore } from '../../stores/uiStore'
import styles from './BitmapView.module.css'
function Line({ line }: { line: BitmapPreviewLine }) {
  const ref = useRef<HTMLCanvasElement>(null)
  const theme = useUiStore((state) => state.theme)
  useEffect(() => {
    const canvas = ref.current
    if (!canvas) return
    const extents = line.glyphs.map((g) =>
      g ? g.bitmap.height + g.bitmap.origin_y : 0,
    )
    const top = Math.max(line.ascent, ...extents),
      bottom = Math.max(
        line.descent,
        ...line.glyphs.map((g) => (g ? -g.bitmap.origin_y : 0)),
      )
    canvas.width = Math.min(
      16384,
      Math.max(
        1,
        line.glyphs.reduce(
          (x, g) => x + Math.max(0, g?.advance ?? line.ppem),
          0,
        ) +
          line.ppem * 2,
      ),
    )
    canvas.height = Math.max(1, Math.min(4096, top + bottom))
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const css = getComputedStyle(canvas),
      paper = css.getPropertyValue('--bm-paper').trim() || '#fdfdfb',
      ink = css.getPropertyValue('--bm-ink').trim() || '#101217'
    ctx.fillStyle = paper
    ctx.fillRect(0, 0, canvas.width, canvas.height)
    let x = line.ppem
    for (const g of line.glyphs) {
      if (g) {
        const image = document.createElement('canvas')
        renderBitmap(
          image,
          g.bitmap,
          1,
          {
            paper,
            ink,
            grid: paper,
            accent: paper,
          },
          undefined,
          false,
        )
        ctx.drawImage(
          image,
          x + g.bitmap.origin_x,
          top - g.bitmap.height - g.bitmap.origin_y,
        )
        x += g.advance
      } else {
        x += line.ppem
      }
    }
  }, [line, theme])
  return (
    <div className={styles.waterfallLine}>
      <span>
        {line.ppem}px · {line.depth}bpp
      </span>
      <canvas ref={ref} />
    </div>
  )
}
/**
 * Every strike's own pixels, for a sample string or for one glyph.
 *
 * Single-glyph mode names the glyph rather than spelling it, so an unencoded
 * glyph — `.notdef`, a stylistic alternate, the reusable pixel glyph — is shown
 * at every size just like an encoded one.
 */
export function BitmapWaterfall({
  doc,
  revision,
  glyph,
}: {
  doc: DocId
  revision: number
  /** Selected glyph name, shown on its own in single-glyph mode. */
  glyph?: string
}) {
  const { t } = useTranslation(),
    [text, setText] = useState('FontCarver 字体点阵 0123456789'),
    [single, setSingle] = useState(false),
    [lines, setLines] = useState<BitmapPreviewLine[]>([])
  const named = single && Boolean(glyph)
  const sample = named ? '' : text
  const selection = named && glyph ? [glyph] : []
  const key = selection.join(' ')
  useEffect(() => {
    let current = true
    const timer = setTimeout(() => {
      const glyphs = key ? key.split(' ') : []
      if (!sample && !glyphs.length) {
        setLines([])
        return
      }
      void call('bitmap.preview', { doc, text: sample, glyphs })
        .then((result) => {
          if (current) setLines(result)
        })
        .catch(reportError)
    }, 150)
    return () => {
      current = false
      clearTimeout(timer)
    }
  }, [doc, sample, key, revision])
  return (
    <section className={styles.waterfall}>
      <label>
        {t('bitmap.waterfall')}
        <input
          value={text}
          disabled={single}
          onChange={(e) => {
            if ([...e.target.value].length <= 128) setText(e.target.value)
          }}
        />
      </label>
      <label className={styles.inline}>
        <input
          type="checkbox"
          checked={single}
          disabled={!glyph}
          onChange={(event) => setSingle(event.target.checked)}
        />
        {glyph
          ? t('bitmap.waterfallGlyph', { glyph })
          : t('bitmap.waterfallGlyphEmpty')}
      </label>
      <p className={styles.hint}>{t('bitmap.waterfallHint')}</p>
      {lines.map((line) => (
        <Line key={`${line.ppem}:${line.depth}`} line={line} />
      ))}
    </section>
  )
}
