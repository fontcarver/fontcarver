import { useTranslation } from 'react-i18next'

import type { DocSummary } from '../../api'
import { Button } from '../../components/Button'
import { Select } from '../../components/Select'
import { Toolbar } from '../../components/Toolbar'
import { useDocumentStore } from '../../stores/documentStore'
import { useEditorStore } from '../../stores/editorStore'

import type { GlyphScene } from './glyphScene'
import styles from './glyphView.module.css'
import { ZOOM_STEP } from './glyphCommands'
import type { GlyphOverlays } from './renderGlyph'

/** Overlay toggles offered in the toolbar, in order. */
const OVERLAYS: readonly (keyof GlyphOverlays)[] = [
  'filled',
  'points',
  'metrics',
  'anchors',
  'guides',
  'rulers',
]

export interface GlyphToolbarProps {
  summary: DocSummary
  scene: GlyphScene
  onFit: () => void
  onZoom: (factor: number) => void
  onStep: (direction: 1 | -1) => void
}

/** Display controls above the glyph canvas. */
export function GlyphToolbar({
  summary,
  scene,
  onFit,
  onZoom,
  onStep,
}: GlyphToolbarProps) {
  const { t } = useTranslation()
  const doc = summary.doc
  const view = useDocumentStore((state) => state.byDoc[doc])
  const setMaster = useDocumentStore((state) => state.setMaster)
  const overlays = useEditorStore((state) => state.overlays)
  const toggleOverlay = useEditorStore((state) => state.toggleOverlay)
  const zoom = useEditorStore((state) => state.zoom)

  const masterId =
    view?.master?.toString() ?? summary.masters[0]?.id.toString() ?? ''
  const pixelsPerEm = Math.max(1, Math.round(zoom * scene.metrics.upem))

  return (
    <Toolbar label={t('glyphView.toolbar')} className={styles.toolbar}>
      <span className={styles.toolbarGroup}>
        <Button
          onClick={() => onStep(-1)}
          aria-label={t('cmd.glyph.previous')}
          title={t('cmd.glyph.previous')}
        >
          ‹
        </Button>
        <Button
          onClick={() => onStep(1)}
          aria-label={t('cmd.glyph.next')}
          title={t('cmd.glyph.next')}
        >
          ›
        </Button>
      </span>

      <span className={styles.toolbarGroup}>
        <span className={styles.toolbarLabel}>{t('inspector.master')}</span>
        <span className={styles.toolbarSelect}>
          <Select
            label={t('inspector.master')}
            value={masterId}
            disabled={summary.masters.length < 2}
            options={summary.masters.map((master) => ({
              value: master.id.toString(),
              label: master.name,
            }))}
            onChange={(value) => setMaster(doc, value)}
          />
        </span>
      </span>

      <span className={styles.toolbarSpacer} />

      <span className={`${styles.toolbarGroup} ${styles.toolbarZoomGroup}`}>
        <Button
          onClick={() => onZoom(1 / ZOOM_STEP)}
          aria-label={t('cmd.view.zoomOut')}
          title={t('cmd.view.zoomOut')}
        >
          −
        </Button>
        <span className={styles.toolbarCount}>
          {t('glyphView.pixelsPerEm', { size: pixelsPerEm })}
        </span>
        <Button
          onClick={() => onZoom(ZOOM_STEP)}
          aria-label={t('cmd.view.zoomIn')}
          title={t('cmd.view.zoomIn')}
        >
          +
        </Button>
        <Button onClick={onFit}>{t('cmd.view.zoomFit')}</Button>
      </span>

      <span
        className={`${styles.toolbarGroup} ${styles.toolbarToggleGroup}`}
        role="group"
        aria-label={t('glyphView.overlays')}
      >
        {OVERLAYS.map((key) => (
          <Button
            key={key}
            aria-pressed={overlays[key]}
            onClick={() => toggleOverlay(key)}
            title={t(`cmd.view.glyph.${key}`)}
          >
            {t(`glyphView.overlay.${key}`)}
          </Button>
        ))}
      </span>
    </Toolbar>
  )
}
