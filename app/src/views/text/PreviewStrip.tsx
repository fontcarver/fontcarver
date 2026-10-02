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
  SlidersHorizontal,
} from 'lucide-react'
import { useId, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { PreviewCanvas } from './PreviewCanvas'
import { PreviewSettings } from './PreviewSettings'
import type { PreviewStripProps } from './presentationTypes'
import { formatWaterfallSizes, MAX_WATERFALL_STEPS } from './waterfallSizes'
import styles from './PreviewStrip.module.css'

export type {
  PreviewGlyph,
  PreviewInstanceOption,
  PreviewMode,
  PreviewSettings,
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
  settings,
  onSettingsChange,
  instances,
}: PreviewStripProps) {
  const { t } = useTranslation()
  const nextTheme = theme === 'light' ? 'dark' : 'light'
  const sizes = settings.sizes
  const canonical = useMemo(() => formatWaterfallSizes(sizes), [sizes])
  const [localHeight, setLocalHeight] = useState(96)
  const resizeStart = useRef<{
    y: number
    height: number
    pointerId: number
  } | null>(null)
  const settingsId = useId()
  const [settingsOpen, setSettingsOpen] = useState(false)
  const minimumHeight = settingsOpen ? 160 : 96
  const previewHeight = Math.max(
    minimumHeight,
    Math.min(
      600,
      Number.isFinite(height) ? (height ?? localHeight) : localHeight,
    ),
  )
  const previewGlyphs = useMemo(() => {
    if (settings.mode === 'run') return glyphs
    const glyph = activeGlyph === undefined ? glyphs[0] : activeGlyph
    return glyph ? [{ ...glyph, x: 0, y: 0, yAdvance: 0 }] : []
  }, [activeGlyph, glyphs, settings.mode])

  function resize(next: number) {
    const bounded = Math.max(minimumHeight, Math.min(600, next))
    setLocalHeight(bounded)
    onHeightChange?.(bounded)
  }

  return (
    <section
      className={`${styles.strip} ${theme === 'light' ? styles.light : styles.dark} ${collapsed ? styles.collapsed : ''}`}
      data-preview-theme={theme}
      data-preview-mode={settings.mode}
      data-preview-sizes={canonical}
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
          aria-valuemin={minimumHeight}
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
                ? minimumHeight
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
          type="button"
          className={styles.iconButton}
          aria-label={t('proofPreview.settings')}
          title={t('proofPreview.settings')}
          aria-expanded={settingsOpen && !collapsed}
          aria-controls={settingsId}
          disabled={collapsed}
          onClick={() => setSettingsOpen(!settingsOpen)}
        >
          <SlidersHorizontal size={14} aria-hidden="true" />
        </button>
        <button
          className={styles.iconButton}
          type="button"
          aria-label={t('proofPreview.flipHorizontal')}
          aria-pressed={settings.flipHorizontal}
          onClick={() =>
            onSettingsChange({ flipHorizontal: !settings.flipHorizontal })
          }
        >
          <FlipHorizontal size={14} aria-hidden="true" />
        </button>
        <button
          className={styles.iconButton}
          type="button"
          aria-label={t('proofPreview.flipVertical')}
          aria-pressed={settings.flipVertical}
          onClick={() =>
            onSettingsChange({ flipVertical: !settings.flipVertical })
          }
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

      <div
        id={settingsId}
        hidden={!settingsOpen || collapsed}
        className={styles.settings}
      >
        <PreviewSettings
          settings={settings}
          onSettingsChange={onSettingsChange}
          instances={instances}
        />
      </div>

      {!collapsed && (
        <>
          <div
            className={styles.previews}
            style={{
              gridTemplateColumns: `repeat(${Math.min(sizes.length, MAX_WATERFALL_STEPS)}, minmax(120px, 1fr))`,
            }}
          >
            {sizes.map((size) => (
              <PreviewCanvas
                key={size}
                glyphs={previewGlyphs}
                unitsPerEm={unitsPerEm}
                size={size}
                theme={theme}
                flipHorizontal={settings.flipHorizontal}
                flipVertical={settings.flipVertical}
                blur={settings.blur}
              />
            ))}
          </div>
        </>
      )}
    </section>
  )
}
