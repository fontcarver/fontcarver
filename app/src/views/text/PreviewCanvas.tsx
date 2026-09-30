import { useCallback, useEffect, useMemo, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import type { PreviewGlyph, PreviewTheme } from './presentationTypes'
import {
  paintPreview,
  PREVIEW_CANVAS_HEIGHT,
  PREVIEW_CANVAS_MAX_WIDTH,
  previewGeometry,
} from './previewPaint'
import styles from './PreviewStrip.module.css'

export interface PreviewCanvasProps {
  glyphs: PreviewGlyph[]
  unitsPerEm: number
  size: number
  theme: PreviewTheme
  flipHorizontal?: boolean
  flipVertical?: boolean
  blur?: number
}

/** One device-pixel-aware preview region at a single true pixel-per-em size. */
export function PreviewCanvas({
  glyphs,
  unitsPerEm,
  size,
  theme,
  flipHorizontal = false,
  flipVertical = false,
  blur = 0,
}: PreviewCanvasProps) {
  const { t } = useTranslation()
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  const safeUnitsPerEm =
    Number.isFinite(unitsPerEm) && unitsPerEm > 0 ? unitsPerEm : 1_000
  const geometry = useMemo(
    () => previewGeometry(glyphs, size, safeUnitsPerEm),
    [glyphs, safeUnitsPerEm, size],
  )

  const attachCanvas = useCallback((node: HTMLCanvasElement | null) => {
    canvasRef.current = node
    return () => {
      canvasRef.current = null
      if (!node) return
      node.width = 0
      node.height = 0
    }
  }, [])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const requestedRatio = Math.max(1, window.devicePixelRatio || 1)
    const pixelRatio = Math.min(
      requestedRatio,
      PREVIEW_CANVAS_MAX_WIDTH / geometry.cssWidth,
      PREVIEW_CANVAS_MAX_WIDTH / (geometry.cssHeight ?? PREVIEW_CANVAS_HEIGHT),
    )
    canvas.width = Math.max(1, Math.round(geometry.cssWidth * pixelRatio))
    canvas.height = Math.max(
      1,
      Math.round((geometry.cssHeight ?? PREVIEW_CANVAS_HEIGHT) * pixelRatio),
    )
    const context = canvas.getContext('2d')
    if (!context) return
    paintPreview(context, glyphs, {
      size,
      unitsPerEm: safeUnitsPerEm,
      width: geometry.cssWidth,
      height: geometry.cssHeight ?? PREVIEW_CANVAS_HEIGHT,
      pixelRatio,
      minX: geometry.minX,
      maxY: geometry.maxY,
      ink: getComputedStyle(canvas).color || 'CanvasText',
      flipHorizontal,
      flipVertical,
      blur,
    })
  }, [
    geometry,
    glyphs,
    safeUnitsPerEm,
    size,
    theme,
    flipHorizontal,
    flipVertical,
    blur,
  ])

  return (
    <div
      className={styles.region}
      role="img"
      aria-label={t('textControls.preview.size', { size })}
    >
      <span className={styles.sizeLabel}>
        {t('textControls.preview.size', { size })}
      </span>
      <div className={styles.scroll} data-preview-scroll>
        <canvas
          ref={attachCanvas}
          className={styles.canvas}
          style={{
            width: geometry.cssWidth,
            height: geometry.cssHeight ?? PREVIEW_CANVAS_HEIGHT,
          }}
          aria-hidden="true"
        />
      </div>
    </div>
  )
}
