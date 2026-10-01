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
import { useEffect, useId, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { PreviewCanvas } from './PreviewCanvas'
import type { PreviewStripProps } from './presentationTypes'
import {
  formatWaterfallSizes,
  parseWaterfallSizes,
  MAX_WATERFALL_STEPS,
  type WaterfallReason,
} from './waterfallSizes'
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
  const [sizeDraft, setSizeDraft] = useState(canonical)
  // What the last attempt to apply the field produced: a refusal reason, a note
  // that a long range was cut, or nothing at all.
  const [sizeNotice, setSizeNotice] = useState<
    WaterfallReason | 'truncated' | null
  >(null)
  const editingSizes = useRef(false)
  // The list this strip itself just applied. The settings coming back are not a
  // new list from elsewhere, so they must not wipe the note that explains them.
  const applied = useRef('')
  const [localHeight, setLocalHeight] = useState(160)
  const resizeStart = useRef<{
    y: number
    height: number
    pointerId: number
  } | null>(null)
  const sizeErrorId = useId()
  // A list arriving from the workspace or the docked strip replaces the draft,
  // but never while it is being typed into.
  useEffect(() => {
    if (editingSizes.current || canonical === applied.current) return
    setSizeDraft(canonical)
    setSizeNotice(null)
  }, [canonical])
  const previewHeight = Math.max(
    120,
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
    const bounded = Math.max(120, Math.min(600, next))
    setLocalHeight(bounded)
    onHeightChange?.(bounded)
  }

  function applySizes() {
    const parsed = parseWaterfallSizes(sizeDraft)
    if (!parsed.sizes) {
      setSizeNotice(parsed.reason)
      return
    }
    editingSizes.current = false
    const canonicalised = formatWaterfallSizes(parsed.sizes)
    applied.current = canonicalised
    setSizeNotice(parsed.truncated ? 'truncated' : null)
    setSizeDraft(canonicalised)
    onSettingsChange({ sizes: parsed.sizes })
  }
  const rejected = sizeNotice !== null && sizeNotice !== 'truncated'

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

      {!collapsed && (
        <>
          <form
            className={styles.controls}
            onSubmit={(event) => {
              event.preventDefault()
              applySizes()
            }}
          >
            {instances && instances.length > 1 && (
              <label>
                {t('proofPreview.instance')}
                <select
                  value={settings.instance}
                  onChange={(event) =>
                    onSettingsChange({ instance: event.target.value })
                  }
                >
                  <optgroup label={t('proofPreview.masters')}>
                    {instances
                      .filter((option) => option.kind === 'master')
                      .map((option) => (
                        <option key={option.id} value={option.id}>
                          {option.name}
                        </option>
                      ))}
                  </optgroup>
                  {instances.some((option) => option.kind === 'instance') && (
                    <optgroup label={t('proofPreview.instances')}>
                      {instances
                        .filter((option) => option.kind === 'instance')
                        .map((option) => (
                          <option key={option.id} value={option.id}>
                            {option.name}
                          </option>
                        ))}
                    </optgroup>
                  )}
                </select>
              </label>
            )}
            <label>
              {t('proofPreview.mode')}
              <select
                value={settings.mode}
                onChange={(event) =>
                  onSettingsChange({
                    mode: event.target.value === 'single' ? 'single' : 'run',
                  })
                }
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
                value={settings.blur}
                onChange={(event) => {
                  const value = Number(event.target.value)
                  onSettingsChange({
                    blur: Number.isFinite(value)
                      ? Math.max(0, Math.min(4, value))
                      : 0,
                  })
                }}
              />
            </label>
            <label>
              {t('proofPreview.sizes')}
              <input
                className={styles.sizesInput}
                value={sizeDraft}
                aria-invalid={rejected}
                aria-describedby={sizeErrorId}
                title={t('proofPreview.sizesHint')}
                onFocus={() => {
                  editingSizes.current = true
                }}
                onBlur={() => {
                  editingSizes.current = false
                }}
                onChange={(event) => {
                  setSizeDraft(event.target.value)
                  setSizeNotice(null)
                }}
              />
            </label>
            <button type="submit">{t('proofPreview.applySizes')}</button>
          </form>
          <p
            id={sizeErrorId}
            className={rejected ? styles.error : styles.hint}
            {...(sizeNotice === null ? {} : { role: 'alert' })}
          >
            {sizeNotice === null
              ? t('proofPreview.sizesHint')
              : sizeNotice === 'truncated'
                ? t('proofPreview.sizesTruncated', {
                    count: MAX_WATERFALL_STEPS,
                  })
                : t(`proofPreview.sizesError.${sizeNotice}`)}
          </p>
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
