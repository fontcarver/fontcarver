import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type ColorPreview as Preview,
  type ColrLayer,
  type DocId,
  type PaletteColor,
} from '../../api'
import { errorNotice } from '../../i18n/errors'
import { formatColor } from './colorModel'
import styles from './color.module.css'

/** Source-over painted default-master outlines, with palette alpha intact. */
export function ColorPreview({
  doc,
  glyph,
  layers,
  palette,
  generation,
}: {
  doc: DocId
  glyph: string
  layers: ColrLayer[]
  palette: PaletteColor[]
  generation: number
}) {
  const { t } = useTranslation()
  const [result, setResult] = useState<{ preview?: Preview; error?: unknown }>(
    {},
  )
  const [foreground, setForeground] = useState('#202020')
  useEffect(() => {
    let stale = false
    const timer = setTimeout(() => {
      void call('color.preview', {
        doc,
        glyph,
        layers,
        palette,
        palette_index: 0,
      })
        .then((preview) => {
          if (!stale) setResult({ preview })
        })
        .catch((error: unknown) => {
          if (!stale) setResult({ error })
        })
    }, 100)
    return () => {
      stale = true
      clearTimeout(timer)
    }
  }, [doc, glyph, layers, palette, generation])
  const preview = result.preview
  const upem = preview?.units_per_em ?? 1000
  const bounds = preview?.layers.map((l) => l.bounds) ?? []
  const left = Math.min(0, ...bounds.map((b) => b[0]))
  const bottom = Math.min(-upem * 0.2, ...bounds.map((b) => b[1]))
  const right = Math.max(preview?.width ?? upem, ...bounds.map((b) => b[2]))
  const top = Math.max(upem * 0.8, ...bounds.map((b) => b[3]))
  const pad = upem * 0.08
  return (
    <figure className={styles.preview}>
      <div className={styles.row}>
        <figcaption>{t('color.preview')}</figcaption>
        <span className={styles.spacer} />
        <label>
          {t('color.previewForeground')}
          <input
            type="color"
            aria-label={t('color.previewForeground')}
            value={foreground}
            onChange={(e) => setForeground(e.target.value)}
          />
        </label>
      </div>
      {result.error ? (
        <p role="alert">{errorNotice(result.error, t).message}</p>
      ) : null}
      <svg
        className={styles.canvas}
        role="img"
        aria-label={t('color.preview')}
        viewBox={`${left - pad} ${-top - pad} ${right - left + 2 * pad} ${top - bottom + 2 * pad}`}
      >
        <g transform="scale(1 -1)">
          {preview?.layers.map((layer, i) => (
            <path
              key={i}
              d={layer.path}
              fill={
                layer.color ? formatColor(layer.color).slice(0, 7) : foreground
              }
              fillOpacity={layer.color ? layer.color.alpha / 255 : 1}
            />
          ))}
        </g>
      </svg>
      <p>{t('color.previewNote')}</p>
    </figure>
  )
}
