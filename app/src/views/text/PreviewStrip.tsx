import {
  AlertTriangle,
  ChevronDown,
  ChevronUp,
  ExternalLink,
  FlipHorizontal,
  FlipVertical,
  Moon,
  Sun,
  X,
} from 'lucide-react'
import { useId, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { PreviewCanvas } from './PreviewCanvas'
import type { PreviewStripProps } from './presentationTypes'
import { PREVIEW_SIZES } from './previewPaint'
import styles from './PreviewStrip.module.css'

export type {
  PreviewGlyph,
  PreviewStatus,
  PreviewStripProps,
  PreviewTheme,
} from './presentationTypes'

/** Controlled outline preview with independent theme and compile status. */
export function PreviewStrip({
  glyphs,
  unitsPerEm,
  status,
  onDismissStatus,
  theme,
  onThemeChange,
  collapsed,
  onCollapsedChange,
  activeGlyph,
  onDetach,
  height,
  onHeightChange,
}: PreviewStripProps) {
  const { t } = useTranslation()
  const nextTheme = theme === 'light' ? 'dark' : 'light'
  const [flipHorizontal, setFlipHorizontal] = useState(false)
  const [flipVertical, setFlipVertical] = useState(false)
  const [blur, setBlur] = useState(0)
  const [mode, setMode] = useState('run')
  const [sizes, setSizes] = useState<number[]>([...PREVIEW_SIZES])
  const [sizeDraft, setSizeDraft] = useState(PREVIEW_SIZES.join(', '))
  const [invalidSizes, setInvalidSizes] = useState(false)
  const [localHeight, setLocalHeight] = useState(160)
  const resizeStart = useRef<{
    y: number
    height: number
    pointerId: number
  } | null>(null)
  const sizeErrorId = useId()
  const previewHeight = Math.max(
    120,
    Math.min(
      600,
      Number.isFinite(height) ? (height ?? localHeight) : localHeight,
    ),
  )
  const previewGlyphs = useMemo(() => {
    if (mode === 'run') return glyphs
    const glyph = activeGlyph === undefined ? glyphs[0] : activeGlyph
    return glyph ? [{ ...glyph, x: 0, y: 0, yAdvance: 0 }] : []
  }, [activeGlyph, glyphs, mode])

  function resize(next: number) {
    const bounded = Math.max(120, Math.min(600, next))
    setLocalHeight(bounded)
    onHeightChange?.(bounded)
  }

  function applySizes() {
    const values = sizeDraft.split(',').map((value) => Number(value.trim()))
    const valid =
      values.length > 0 &&
      values.length <= 8 &&
      values.every(
        (value) => Number.isFinite(value) && value >= 6 && value <= 256,
      ) &&
      new Set(values).size === values.length
    setInvalidSizes(!valid)
    if (valid) setSizes(values)
  }

  return (
    <section
      className={`${styles.strip} ${theme === 'light' ? styles.light : styles.dark} ${collapsed ? styles.collapsed : ''}`}
      data-preview-theme={theme}
      aria-label={t('textControls.preview.label')}
      style={collapsed ? undefined : { height: previewHeight }}
    >
      {!collapsed && (
        <div
          className={styles.resizeHandle}
          role="separator"
          tabIndex={0}
          aria-label={t('proofPreview.resize')}
          aria-orientation="horizontal"
          aria-valuemin={120}
          aria-valuemax={600}
          aria-valuenow={previewHeight}
          onPointerDown={(event) => {
            if (event.button !== 0) return
            event.preventDefault()
            event.currentTarget.focus()
            event.currentTarget.setPointerCapture?.(event.pointerId)
            resizeStart.current = {
              y: event.clientY,
              height: previewHeight,
              pointerId: event.pointerId,
            }
          }}
          onPointerMove={(event) => {
            const start = resizeStart.current
            if (start?.pointerId === event.pointerId)
              resize(start.height + start.y - event.clientY)
          }}
          onPointerUp={(event) => {
            if (resizeStart.current?.pointerId !== event.pointerId) return
            event.currentTarget.releasePointerCapture?.(event.pointerId)
            resizeStart.current = null
          }}
          onPointerCancel={() => {
            resizeStart.current = null
          }}
          onLostPointerCapture={() => {
            resizeStart.current = null
          }}
          onKeyDown={(event) => {
            const next =
              event.key === 'Home'
                ? 120
                : event.key === 'End'
                  ? 600
                  : event.key === 'ArrowUp'
                    ? previewHeight + 10
                    : event.key === 'ArrowDown'
                      ? previewHeight - 10
                      : null
            if (next === null) return
            event.preventDefault()
            resize(next)
          }}
        />
      )}
      <div className={styles.header}>
        <span className={styles.title}>{t('textControls.preview.label')}</span>
        {status !== 'ready' && (
          <span
            className={`${styles.status} ${status === 'compile-failed' ? styles.failed : ''}`}
            role="status"
          >
            {status === 'stale' ? (
              <span className={styles.statusDot} aria-hidden="true" />
            ) : (
              <AlertTriangle size={13} aria-hidden="true" />
            )}
            {status === 'stale'
              ? t('textControls.preview.stale')
              : t('textControls.preview.compileFailed')}
            <button
              className={styles.iconButton}
              type="button"
              aria-label={t('textControls.preview.dismissStatus')}
              onClick={onDismissStatus}
            >
              <X size={13} aria-hidden="true" />
            </button>
          </span>
        )}
        <span className={styles.spacer} />
        <button
          className={styles.iconButton}
          type="button"
          aria-label={t('proofPreview.flipHorizontal')}
          aria-pressed={flipHorizontal}
          onClick={() => setFlipHorizontal(!flipHorizontal)}
        >
          <FlipHorizontal size={14} aria-hidden="true" />
        </button>
        <button
          className={styles.iconButton}
          type="button"
          aria-label={t('proofPreview.flipVertical')}
          aria-pressed={flipVertical}
          onClick={() => setFlipVertical(!flipVertical)}
        >
          <FlipVertical size={14} aria-hidden="true" />
        </button>
        <button
          className={styles.iconButton}
          type="button"
          aria-label={
            nextTheme === 'light'
              ? t('textControls.preview.useLight')
              : t('textControls.preview.useDark')
          }
          aria-pressed={theme === 'dark'}
          onClick={() => onThemeChange(nextTheme)}
        >
          {theme === 'light' ? (
            <Moon size={14} aria-hidden="true" />
          ) : (
            <Sun size={14} aria-hidden="true" />
          )}
        </button>
        {onDetach && (
          <button
            className={styles.iconButton}
            type="button"
            aria-label={t('proofPreview.detach')}
            onClick={onDetach}
          >
            <ExternalLink size={14} aria-hidden="true" />
          </button>
        )}
        <button
          className={styles.iconButton}
          type="button"
          aria-label={
            collapsed
              ? t('textControls.preview.expand')
              : t('textControls.preview.collapse')
          }
          aria-expanded={!collapsed}
          onClick={() => onCollapsedChange(!collapsed)}
        >
          {collapsed ? (
            <ChevronUp size={14} aria-hidden="true" />
          ) : (
            <ChevronDown size={14} aria-hidden="true" />
          )}
        </button>
      </div>

      {!collapsed && (
        <>
          <form
            className={styles.controls}
            onSubmit={(event) => {
              event.preventDefault()
              applySizes()
            }}
          >
            <label>
              {t('proofPreview.mode')}
              <select
                value={mode}
                onChange={(event) => setMode(event.target.value)}
              >
                <option value="run">{t('proofPreview.run')}</option>
                <option value="single">{t('proofPreview.single')}</option>
              </select>
            </label>
            <label>
              {t('proofPreview.blur')}
              <input
                className={styles.blurInput}
                type="number"
                min={0}
                max={4}
                step={0.25}
                value={blur}
                onChange={(event) => {
                  const value = Number(event.target.value)
                  setBlur(
                    Number.isFinite(value)
                      ? Math.max(0, Math.min(4, value))
                      : 0,
                  )
                }}
              />
            </label>
            <label>
              {t('proofPreview.sizes')}
              <input
                className={styles.sizesInput}
                value={sizeDraft}
                aria-invalid={invalidSizes}
                aria-describedby={invalidSizes ? sizeErrorId : undefined}
                onChange={(event) => setSizeDraft(event.target.value)}
              />
            </label>
            <button type="submit">{t('proofPreview.applySizes')}</button>
          </form>
          {invalidSizes && (
            <p id={sizeErrorId} className={styles.error} role="alert">
              {t('proofPreview.sizesHint')}
            </p>
          )}
          <div
            className={styles.previews}
            style={{
              gridTemplateColumns: `repeat(${sizes.length}, minmax(120px, 1fr))`,
            }}
          >
            {sizes.map((size) => (
              <PreviewCanvas
                key={size}
                glyphs={previewGlyphs}
                unitsPerEm={unitsPerEm}
                size={size}
                theme={theme}
                flipHorizontal={flipHorizontal}
                flipVertical={flipVertical}
                blur={blur}
              />
            ))}
          </div>
        </>
      )}
    </section>
  )
}
