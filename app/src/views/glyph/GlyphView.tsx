import { useCallback, useEffect, useMemo, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocSummary, GlyphId } from '../../api'
import { replaceGlyphTab } from '../../shell/tabActions'
import { useDocumentStore, selectDocView } from '../../stores/documentStore'
import { useEditorStore } from '../../stores/editorStore'
import { useFontInfo } from '../../stores/fontInfoCache'
import { tabId, useUiStore } from '../../stores/uiStore'
import { neighbourGid } from '../font/glyphOrder'
import type { GlyphQuery } from '../font/glyphSelection'

import { GlyphCanvas, type GlyphCanvasHandle } from './GlyphCanvas'
import { GlyphHud } from './GlyphHud'
import { GlyphToolbar } from './GlyphToolbar'
import { registerGlyphView } from './glyphCommands'
import { buildScene, formatUnicodes } from './glyphScene'
import { useGlyphTheme } from './glyphTheme'
import styles from './glyphView.module.css'
import { useGlyphDetail } from './useGlyphDetail'
import { useMetricLabels } from './useMetricLabels'

export interface GlyphViewProps {
  summary: DocSummary
  gid: GlyphId | undefined
}

/**
 * The Glyph tab: a read-only outline canvas with its metrics, guides, anchors
 * and components, plus the toolbar and the corner readout. Editing tools
 * arrive in the next phase; everything here is display only.
 */
export function GlyphView({ summary, gid }: GlyphViewProps) {
  const { t } = useTranslation()
  const doc = summary.doc
  const view = useDocumentStore(selectDocView(doc))
  const masterId = view.master ?? summary.masters[0]?.id ?? null
  const overlays = useEditorStore((state) => state.overlays)
  const setPointer = useEditorStore((state) => state.setPointer)
  const setZoom = useEditorStore((state) => state.setZoom)
  const setActiveGlyph = useEditorStore((state) => state.setActiveGlyph)
  const toggleOverlay = useEditorStore((state) => state.toggleOverlay)
  const setTabLabel = useUiStore((state) => state.setTabLabel)

  const detail = useGlyphDetail(doc, gid, masterId)
  const fontInfo = useFontInfo(doc, detail.fontGeneration)
  const theme = useGlyphTheme()
  const labels = useMetricLabels()
  const handleRef = useRef<GlyphCanvasHandle | null>(null)

  const master = useMemo(() => {
    const masters = fontInfo.info?.masters ?? []
    const wanted = masterId ?? ''
    return masters.find((entry) => entry.id === wanted) ?? masters[0] ?? null
  }, [fontInfo.info, masterId])

  const scene = useMemo(
    () =>
      detail.detail === null
        ? null
        : buildScene({
            detail: detail.detail,
            cell: detail.cell,
            master,
            masterId,
            unitsPerEm: summary.units_per_em,
          }),
    [detail.detail, detail.cell, master, masterId, summary.units_per_em],
  )

  const flattenedData = scene?.flattened ?? null
  const flattened = useMemo(() => {
    if (!flattenedData) return null
    try {
      return new Path2D(flattenedData)
    } catch {
      // Malformed path data must not take the whole view down.
      return null
    }
  }, [flattenedData])

  const query = useMemo<GlyphQuery>(
    () => ({
      doc,
      master: masterId,
      filter: view.filter,
      sort: view.sort,
    }),
    [doc, masterId, view.filter, view.sort],
  )

  // Publish the glyph so the shell inspector can render it without a second
  // request, and name the tab now that the glyph name is known.
  useEffect(() => {
    if (!scene || gid === undefined || !detail.detail) {
      setActiveGlyph(null)
      return
    }
    setActiveGlyph({
      doc,
      gid,
      glyph: detail.detail.glyph,
      scene,
      cell: detail.cell,
    })
    setTabLabel(tabId('glyph', doc, gid), scene.name)
    return () => setActiveGlyph(null)
  }, [detail.cell, detail.detail, doc, gid, scene, setActiveGlyph, setTabLabel])

  useEffect(() => () => setPointer(null), [setPointer])

  const step = useCallback(
    (direction: 1 | -1) => {
      if (gid === undefined) return
      void neighbourGid(query, gid, direction).then((next) => {
        if (next !== null && next !== gid) replaceGlyphTab(doc, gid, next)
      })
    },
    [doc, gid, query],
  )

  useEffect(() => {
    if (gid === undefined) return
    return registerGlyphView({
      doc,
      gid,
      fit: () => handleRef.current?.fit(),
      zoomBy: (factor) => handleRef.current?.zoomBy(factor),
      step,
    })
  }, [doc, gid, step])

  const toggleFilled = useCallback(
    () => toggleOverlay('filled'),
    [toggleOverlay],
  )

  if (gid === undefined)
    return <p className={styles.message}>{t('glyphView.noGlyph')}</p>
  if (detail.error) return <p className={styles.message}>{detail.error}</p>
  if (!scene) return <p className={styles.message}>{t('glyphView.loading')}</p>

  return (
    <div className={styles.view}>
      <GlyphToolbar
        summary={summary}
        scene={scene}
        onFit={() => handleRef.current?.fit()}
        onZoom={(factor) => handleRef.current?.zoomBy(factor)}
        onStep={step}
      />
      <div className={styles.stage}>
        <GlyphCanvas
          scene={scene}
          theme={theme}
          labels={labels}
          overlays={overlays}
          flattened={flattened}
          fitKey={`${doc}:${gid}:${masterId ?? ''}`}
          label={t('glyphView.canvasLabel', {
            name: scene.name,
            unicode: formatUnicodes(scene.unicodes),
          })}
          onPointerFont={setPointer}
          onZoomChange={setZoom}
          onToggleFilled={toggleFilled}
          handleRef={handleRef}
        />
        <GlyphHud scene={scene} />
      </div>
    </div>
  )
}
