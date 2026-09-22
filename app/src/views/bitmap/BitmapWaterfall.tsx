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
      paper = css.getPropertyValue('--color-canvas').trim() || '#fff',
      ink = css.getPropertyValue('--color-text').trim() || '#171717'
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
export function BitmapWaterfall({
  doc,
  revision,
}: {
  doc: DocId
  revision: number
}) {
  const { t } = useTranslation(),
    [text, setText] = useState('FontCarver 字体点阵 0123456789'),
    [lines, setLines] = useState<BitmapPreviewLine[]>([])
  useEffect(() => {
    let current = true
    const timer = setTimeout(() => {
      void call('bitmap.preview', { doc, text })
        .then((result) => {
          if (current) setLines(result)
        })
        .catch(reportError)
    }, 150)
    return () => {
      current = false
      clearTimeout(timer)
    }
  }, [doc, text, revision])
  return (
    <section className={styles.waterfall}>
      <label>
        {t('bitmap.waterfall')}
        <input
          value={text}
          onChange={(e) => {
            if ([...e.target.value].length <= 128) setText(e.target.value)
          }}
        />
      </label>
      <p className={styles.hint}>{t('bitmap.waterfallHint')}</p>
      {lines.map((line) => (
        <Line key={`${line.ppem}:${line.depth}`} line={line} />
      ))}
    </section>
  )
}
