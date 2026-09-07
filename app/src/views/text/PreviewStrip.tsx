import {
  AlertTriangle,
  ChevronDown,
  ChevronUp,
  Moon,
  Sun,
  X,
} from 'lucide-react'
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
}: PreviewStripProps) {
  const { t } = useTranslation()
  const nextTheme = theme === 'light' ? 'dark' : 'light'

  return (
    <section
      className={`${styles.strip} ${theme === 'light' ? styles.light : styles.dark} ${collapsed ? styles.collapsed : ''}`}
      data-preview-theme={theme}
      aria-label={t('textControls.preview.label')}
    >
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
        <div className={styles.previews}>
          {PREVIEW_SIZES.map((size) => (
            <PreviewCanvas
              key={size}
              glyphs={glyphs}
              unitsPerEm={unitsPerEm}
              size={size}
              theme={theme}
            />
          ))}
        </div>
      )}
    </section>
  )
}
