import { useEffect, useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type ColorTable, type DocId } from '../../api'
import { errorNotice } from '../../i18n/errors'
import styles from './color.module.css'

/** The browser renders all static COLRv1 paints from an isolated draft font. */
export function ColorGraphPreview({
  doc,
  glyph,
  color,
  palette,
  generation,
}: {
  doc: DocId
  glyph: string
  color: ColorTable
  palette: number
  generation: number
}) {
  const { t } = useTranslation()
  const id = useId().replace(/[^a-z0-9]/gi, '')
  const canvas = useRef<HTMLCanvasElement>(null)
  const [foreground, setForeground] = useState('#202020')
  const [status, setStatus] = useState<{ key: string; error: unknown }>({
    key: '',
    error: null,
  })
  const key = JSON.stringify([
    doc,
    glyph,
    color,
    palette,
    generation,
    foreground,
  ])
  useEffect(() => {
    let active = true
    let face: FontFace | undefined
    const timer = setTimeout(() => {
      void (async () => {
        const result = await call('color.graph_preview', {
          doc,
          glyph,
          color,
          palette_index: palette,
        })
        if (!active) return
        if (
          typeof FontFace === 'undefined' ||
          !CSS.supports('font-tech(color-COLRv1)')
        )
          throw new Error(t('color.graphRendererUnavailable'))
        face = new FontFace(`fc-colrv1-${id}`, new Uint8Array(result.bytes))
        await face.load()
        if (!active) return
        document.fonts.add(face)
        const ctx = canvas.current?.getContext('2d')
        if (!ctx) throw new Error(t('color.graphRendererUnavailable'))
        ctx.clearRect(0, 0, 800, 440)
        ctx.font = `200px fc-colrv1-${id}`
        const m = ctx.measureText('\uE000')
        const left = Math.max(0, m.actualBoundingBoxLeft)
        const right = Math.max(m.width, m.actualBoundingBoxRight, 1)
        const top = Math.max(0, m.actualBoundingBoxAscent)
        const bottom = Math.max(0, m.actualBoundingBoxDescent)
        const scale = Math.min(
          720 / (left + right),
          380 / Math.max(1, top + bottom),
        )
        ctx.save()
        ctx.translate(
          (800 - (left + right) * scale) / 2 + left * scale,
          (440 - (top + bottom) * scale) / 2 + top * scale,
        )
        ctx.scale(scale, scale)
        ctx.fillStyle = foreground
        ctx.fillText('\uE000', 0, 0)
        ctx.restore()
        setStatus({ key, error: null })
      })().catch((error: unknown) => {
        if (active) setStatus({ key, error })
      })
    }, 300)
    return () => {
      active = false
      clearTimeout(timer)
      if (face) document.fonts.delete(face)
    }
  }, [doc, glyph, color, palette, generation, foreground, id, key, t])
  const ready = status.key === key
  return (
    <figure className={styles.preview}>
      <figcaption>{t('color.graphPreview')}</figcaption>
      <label className={styles.row}>
        {t('color.previewForeground')}
        <input
          type="color"
          value={foreground}
          onChange={(e) => setForeground(e.target.value)}
        />
      </label>
      <canvas
        ref={canvas}
        width={800}
        height={440}
        className={styles.canvas}
        hidden={!ready || !!status.error}
        role="img"
        aria-label={t('color.graphPreview')}
      />
      {!ready && <p role="status">{t('color.graphLoading')}</p>}
      {ready && !!status.error && (
        <p role="alert">{errorNotice(status.error, t).message}</p>
      )}
      <p>{t('color.graphPreviewNote')}</p>
    </figure>
  )
}
