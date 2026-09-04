import { useCallback, useEffect, useMemo, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocSummary, GlyphId } from '../../api'
import type { EditorSource } from '../../editor/editorController'
import type { ScopedGuide } from '../../editor/hitTest'
import { replaceGlyphTab } from '../../shell/tabActions'
import { useDocumentStore, selectDocView } from '../../stores/documentStore'
import { useEditorStore } from '../../stores/editorStore'
import { useFontInfo } from '../../stores/fontInfoCache'
import { tabId, useUiStore } from '../../stores/uiStore'
import { neighbourGid } from '../font/glyphOrder'
import type { GlyphQuery } from '../font/glyphSelection'

import { GlyphCanvas, type GlyphCanvasHandle } from './GlyphCanvas'
import { GlyphToolbar } from './GlyphToolbar'
import { InfoBox } from './InfoBox'
import { ToolPalette } from './ToolPalette'
import { registerGlyphView } from './glyphCommands'
import { buildScene, formatUnicodes } from './glyphScene'
import { useGlyphTheme } from './glyphTheme'
import styles from './glyphView.module.css'
import { useEditorLabels } from './useEditorLabels'
import { useGlyphDetail } from './useGlyphDetail'
import { useLayerOutline } from './useLayerOutline'
import { useMetricLabels } from './useMetricLabels'
import { useToolShortcuts } from './useToolShortcuts'

export interface GlyphViewProps {
  summary: DocSummary
  gid: GlyphId | undefined
}

/**
 * The Edit tab: the tool strip, the editable outline canvas with its metrics,
 * guides, anchors and components, and the docked Info box.
 *
 * The drawn geometry comes from `glyphs.outline` — the same stored contours
 * every editing command indexes — so what is on screen and what a command
 * addresses cannot drift apart.
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
  const selection = useEditorStore((state) => state.selection)
  const setTabLabel = useUiStore((state) => state.setTabLabel)

  const detail = useGlyphDetail(doc, gid, masterId)
  const fontInfo = useFontInfo(doc, detail.fontGeneration)
  const theme = useGlyphTheme()
  const labels = useMetricLabels()
  const editorLabels = useEditorLabels()
  const handleRef = useRef<GlyphCanvasHandle | null>(null)
  useToolShortcuts(gid !== undefined)

  const master = useMemo(() => {
    const masters = fontInfo.info?.masters ?? []
    const wanted = masterId ?? ''
    return masters.find((entry) => entry.id === wanted) ?? masters[0] ?? null
  }, [fontInfo.info, masterId])

  const baseScene = useMemo(
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

  const outlineResult = useLayerOutline(doc, gid, baseScene?.layer?.id ?? null)
  const outline = outlineResult.outline

  // The editable outline replaces the display copy as soon as it arrives, so
  // the canvas draws exactly the geometry the commands address.
  const scene = useMemo(() => {
    if (!baseScene) return null
    if (!outline) return baseScene
    return {
      ...baseScene,
      contours: outline.contours,
      anchors: outline.anchors,
      guides: [
        ...(master?.guides ?? []).map((guide) => ({
          guide,
          scope: 'master' as const,
        })),
        ...outline.guides.map((guide) => ({
          guide,
          scope: 'layer' as const,
        })),
      ],
      metrics: {
        ...baseScene.metrics,
        width: outline.width,
        lsb: outline.bbox ? outline.bbox[0] : baseScene.metrics.lsb,
        rsb: outline.bbox
          ? outline.width - outline.bbox[2]
          : baseScene.metrics.rsb,
      },
    }
  }, [baseScene, master, outline])

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

  const source = useMemo<EditorSource | null>(() => {
    if (!scene || !outline || gid === undefined) return null
    return {
      doc,
      gid,
      glyph: scene.name,
      outline,
      guides: scene.guides as readonly ScopedGuide[],
      metrics: scene.metrics,
      master: masterId,
    }
  }, [doc, gid, masterId, outline, scene])

  const query = useMemo<GlyphQuery>(
    () => ({ doc, master: masterId, filter: view.filter, sort: view.sort }),
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
  if (outlineResult.error)
    return <p className={styles.message}>{outlineResult.error}</p>
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
        <ToolPalette />
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
          source={source}
          editorLabels={editorLabels}
          onPointerFont={setPointer}
          onZoomChange={setZoom}
          onToggleFilled={toggleFilled}
          onReload={outlineResult.reload}
          handleRef={handleRef}
        />
        <p className={styles.liveRegion} aria-live="polite" role="status">
          {t('editor.a11y.selection', { count: selection.size })}
        </p>
        {outline ? (
          <InfoBox
            target={{ doc, glyph: scene.name, layer: outline.layer }}
            outline={outline}
            selection={selection}
            master={masterId}
            name={scene.name}
            unicodes={scene.unicodes}
            onCommitted={outlineResult.reload}
          />
        ) : null}
      </div>
    </div>
  )
}
