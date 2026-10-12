import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { TextCanvasRun } from '../text/textCanvas'
import type { DocSummary, GlyphId } from '../../api'
import type { EditorSource } from '../../editor/editorController'
import { activeEditor } from '../../editor/activeEditor'
import type { ScopedGuide } from '../../editor/hitTest'
import { changeGlyphTab } from '../../shell/tabActions'
import { inferCurves } from '../../editor/pathGeometry'
import { useDocumentStore, selectDocView } from '../../stores/documentStore'
import { useEditorStore } from '../../stores/editorStore'
import {
  nudgeStepsOf,
  snapSettingsOf,
  usePrefsStore,
} from '../../stores/prefsStore'
import { useFontInfo } from '../../stores/fontInfoCache'
import { tabId, useUiStore } from '../../stores/uiStore'
import { neighbourGid } from '../font/glyphOrder'
import type { GlyphQuery } from '../font/glyphSelection'

import { GlyphCanvas, type GlyphCanvasHandle } from './GlyphCanvas'
import { NO_UNDERLAY, type UnderlayData } from './drawUnderlays'
import { GlyphToolbar } from './GlyphToolbar'
import { InfoBox } from './InfoBox'
import { ToolPalette } from './ToolPalette'
import { ShapeDialog } from './ShapeDialog'
import { registerGlyphView } from './glyphCommands'
import { buildScene, formatUnicodes } from './glyphScene'
import { inkEdgesOf } from './guideAnchor'
import { useGlyphTheme } from './glyphTheme'
import styles from './glyphView.module.css'
import { useEditorLabels } from './useEditorLabels'
import { useFontEditorSettings } from './useFontEditorSettings'
import { useGlyphDetail } from './useGlyphDetail'
import { useLayerOutline } from './useLayerOutline'
import { useMetricLabels } from './useMetricLabels'
import { useComponentPoints } from './useComponentPoints'
import { usePixelHandover } from './pixelHandover'
import { useToolShortcuts } from './useToolShortcuts'
import { layerMaster } from './panels/layerRows'

export interface GlyphViewProps {
  summary: DocSummary
  gid: GlyphId | undefined
  tabIdentity?: string
  textRun?: TextCanvasRun | null
  textFitKey?: string
  externalSettler?: (() => Promise<void>) | null
}

/**
 * The Edit tab: the tool strip, the editable outline canvas with its metrics,
 * guides, anchors and components, and the docked Info box.
 *
 * The drawn geometry comes from `glyphs.outline` — the same stored contours
 * every editing command indexes — so what is on screen and what a command
 * addresses cannot drift apart.
 */
export function GlyphView({
  summary,
  gid,
  tabIdentity,
  textRun = null,
  textFitKey,
  externalSettler,
}: GlyphViewProps) {
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
  const preview = useEditorStore((state) => state.preview)
  const editedLayer = useEditorStore((state) => state.editedLayer)
  const setEditedLayer = useEditorStore((state) => state.setEditedLayer)
  const shapeDialog = useEditorStore((state) => state.shapeDialog)
  const setShapeDialog = useEditorStore((state) => state.setShapeDialog)
  const prefs = usePrefsStore((state) => state.prefs)
  const loadPrefs = usePrefsStore((state) => state.load)
  const setTabLabel = useUiStore((state) => state.setTabLabel)

  const detail = useGlyphDetail(doc, gid, masterId)
  const fontInfo = useFontInfo(doc, detail.fontGeneration)
  const theme = useGlyphTheme()
  const labels = useMetricLabels()
  const editorLabels = useEditorLabels()
  const handleRef = useRef<GlyphCanvasHandle | null>(null)
  useToolShortcuts(gid !== undefined && textRun?.mode !== 'text')
  usePixelHandover(doc, gid !== undefined)

  const editingMasterId = useMemo(() => {
    if (!editedLayer) return masterId
    const layer = detail.detail?.glyph.layers.find(
      (entry) => entry.id === editedLayer,
    )
    return layer ? (layerMaster(layer.kind) ?? masterId) : masterId
  }, [detail.detail, editedLayer, masterId])

  const master = useMemo(() => {
    const masters = fontInfo.info?.masters ?? []
    const wanted = editingMasterId ?? ''
    return masters.find((entry) => entry.id === wanted) ?? masters[0] ?? null
  }, [editingMasterId, fontInfo.info])

  // A layer chosen in the Layers panel belongs to the glyph it was chosen in;
  // opening another glyph goes back to that glyph's master layer.
  useEffect(() => {
    setEditedLayer(null)
  }, [doc, gid, setEditedLayer])

  const baseScene = useMemo(
    () =>
      detail.detail === null
        ? null
        : buildScene({
            detail: detail.detail,
            cell: detail.cell,
            master,
            masterId: editingMasterId,
            unitsPerEm: summary.units_per_em,
            layerId: editedLayer,
          }),
    [
      detail.detail,
      detail.cell,
      editedLayer,
      master,
      editingMasterId,
      summary.units_per_em,
    ],
  )

  const outlineResult = useLayerOutline(doc, gid, baseScene?.layer?.id ?? null)
  const outline = outlineResult.outline
  const componentPoints = useComponentPoints(
    doc,
    gid,
    outline?.layer ?? null,
    outline?.components.length ?? 0,
  )

  // The kernel publishes no prefs-changed event, so they are read whenever a
  // glyph tab mounts, which is the moment the editor needs them.
  useEffect(() => {
    void loadPrefs()
  }, [loadPrefs])

  // The editable outline replaces the display copy as soon as it arrives, so
  // the canvas draws exactly the geometry the commands address.
  const scene = useMemo(() => {
    if (!baseScene) return null
    if (!outline) return baseScene
    return {
      ...baseScene,
      contours: outline.contours,
      anchors: outline.anchors,
      components: outline.components.map((view) => view.component),
      componentBounds: outline.components.map((view) => view.bbox),
      guides: [
        ...(master?.guides ?? []).map((guide, index) => ({
          guide,
          scope: 'master' as const,
          index,
        })),
        ...outline.guides.map((guide, index) => ({
          guide,
          scope: 'layer' as const,
          index,
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

  const underlay = useMemo<UnderlayData>(() => {
    if (!scene) return NO_UNDERLAY
    const parse = (data: string | null): Path2D | null => {
      if (!data) return null
      try {
        return new Path2D(data)
      } catch {
        // Malformed path data must not take the whole view down.
        return null
      }
    }
    return {
      background: parse(scene.background?.flattened ?? null),
      backgroundPoints: (scene.background?.contours ?? []).flatMap((contour) =>
        contour.points.map((point) => ({ x: point.x, y: point.y })),
      ),
      otherMasters: scene.otherMasters
        .map(parse)
        .filter((path): path is Path2D => path !== null),
    }
  }, [scene])

  const source = useMemo<EditorSource | null>(() => {
    if (!scene || !outline || gid === undefined) return null
    return {
      doc,
      gid,
      glyph: scene.name,
      outline,
      guides: scene.guides as readonly ScopedGuide[],
      metrics: scene.metrics,
      // The effective vertical metrics are in `scene.metrics`; the canvas's
      // vertical handles also need what this layer itself stores, so dragging
      // one field leaves an inherited one inherited.
      verticalOverrides: {
        origin: scene.layer?.vertical_origin ?? null,
        advance: scene.layer?.vertical_width ?? null,
      },
      master: outline.master,
      curves: inferCurves(
        outline.contours,
        summary.format,
        outline.curve_format,
      ),
      componentPoints,
      zoneLines: (master?.alignment_zones ?? []).flatMap((zone) => [
        zone.position,
        zone.position + zone.size,
      ]),
      backgroundPoints: underlay.backgroundPoints,
    }
  }, [
    componentPoints,
    doc,
    gid,
    master,
    outline,
    scene,
    summary.format,
    underlay.backgroundPoints,
  ])

  // The font's own grid and increments win over the application preference;
  // the kernel has already resolved that order, so this only applies it.
  const fontEditor = useFontEditorSettings(doc, detail.fontGeneration)
  const snapSettings = useMemo(
    () => snapSettingsOf(prefs, fontEditor),
    [fontEditor, prefs],
  )
  const nudgeSteps = useMemo(
    () => nudgeStepsOf(prefs, fontEditor),
    [fontEditor, prefs],
  )

  const query = useMemo<GlyphQuery>(
    () => ({
      doc,
      master: editingMasterId,
      filter: view.filter,
      sort: view.sort,
    }),
    [doc, editingMasterId, view.filter, view.sort],
  )

  const textMode = textRun?.mode
  const hasTextRun = Boolean(textRun)

  // Publish the glyph so the shell inspector can render it without a second
  // request, and name the tab now that the glyph name is known.
  useEffect(() => {
    if (!scene || gid === undefined || !detail.detail || textMode === 'text') {
      setActiveGlyph(null)
      return
    }
    setActiveGlyph({
      doc,
      gid,
      glyph: detail.detail.glyph,
      scene,
      cell: detail.cell,
      outline,
      master: outline?.master ?? editingMasterId,
      masterGuides: master?.guides ?? [],
      reload: outlineResult.reload,
    })
    if (!hasTextRun)
      setTabLabel(tabIdentity ?? tabId('glyph', doc, gid), scene.name)
    return () => setActiveGlyph(null)
  }, [
    detail.cell,
    detail.detail,
    doc,
    gid,
    master,
    editingMasterId,
    outline,
    outlineResult.reload,
    scene,
    setActiveGlyph,
    setTabLabel,
    tabIdentity,
    textMode,
    hasTextRun,
  ])

  useEffect(() => () => setPointer(null), [setPointer])

  const step = useCallback(
    (direction: 1 | -1) => {
      if (gid === undefined) return
      const originatingTab = useUiStore.getState().activeTab
      if (originatingTab === null) return
      void neighbourGid(query, gid, direction).then((next) => {
        if (next !== null && next !== gid)
          return changeGlyphTab(doc, gid, next, undefined, originatingTab)
      })
    },
    [doc, gid, query],
  )

  const changeMaster = useCallback(
    async (master: string) => {
      if (gid === undefined) return
      const id = tabIdentity ?? tabId('glyph', doc, gid)
      const current = () => {
        const ui = useUiStore.getState()
        const tab = ui.tabs.find((tab) => tab.id === id)
        return ui.activeTab === id && tab?.doc === doc && tab.gid === gid
      }
      if (!current()) return
      const editor = activeEditor()
      await editor?.prepareTransition()
      if (!current() || activeEditor() !== editor) return
      setEditedLayer(null)
      useDocumentStore.getState().setMaster(doc, master)
    },
    [doc, gid, setEditedLayer, tabIdentity],
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

  const [lastScene, setLastScene] = useState(scene)
  if (scene && lastScene !== scene) setLastScene(scene)
  const shownScene = scene ?? (textRun ? lastScene : null)

  if (gid === undefined && !textRun)
    return <p className={styles.message}>{t('glyphView.noGlyph')}</p>
  if (detail.error) return <p className={styles.message}>{detail.error}</p>
  if (outlineResult.error)
    return <p className={styles.message}>{outlineResult.error}</p>
  if (!shownScene)
    return <p className={styles.message}>{t('glyphView.loading')}</p>

  return (
    <div className={styles.view}>
      <GlyphToolbar
        summary={summary}
        scene={shownScene}
        onFit={() => handleRef.current?.fit()}
        onZoom={(factor) => handleRef.current?.zoomBy(factor)}
        onStep={step}
        onMasterChange={changeMaster}
      />
      <div className={styles.stage}>
        <ToolPalette />
        <GlyphCanvas
          scene={shownScene}
          theme={theme}
          labels={labels}
          overlays={overlays}
          flattened={flattened}
          underlay={underlay}
          componentViews={outline?.components ?? []}
          fitKey={textFitKey ?? `${doc}:${gid}:${editingMasterId ?? ''}`}
          textRun={textRun}
          externalSettler={externalSettler}
          label={t('glyphView.canvasLabel', {
            name: shownScene.name,
            unicode: formatUnicodes(shownScene.unicodes),
          })}
          source={textRun?.mode === 'text' ? null : source}
          editorLabels={editorLabels}
          snapSettings={snapSettings}
          nudgeSteps={nudgeSteps}
          onShapeDialog={setShapeDialog}
          onPointerFont={setPointer}
          onZoomChange={setZoom}
          onToggleFilled={toggleFilled}
          onReload={outlineResult.reload}
          handleRef={handleRef}
        />
        <p className={styles.liveRegion} aria-live="polite" role="status">
          {t('editor.a11y.selection', { count: selection.size })}
        </p>
        {outline && textRun?.mode !== 'text' ? (
          <InfoBox
            target={{ doc, glyph: shownScene.name, layer: outline.layer }}
            // While a gesture runs the fetched outline is deliberately stale,
            // so the box reads the geometry the canvas is drawing.
            outline={
              preview
                ? {
                    ...outline,
                    contours: preview.contours,
                    anchors: preview.anchors,
                    components: preview.components,
                  }
                : outline
            }
            selection={selection}
            master={outline.master}
            masterGuides={master?.guides ?? []}
            inkEdges={inkEdgesOf(shownScene.metrics)}
            name={shownScene.name}
            unicodes={shownScene.unicodes}
            onCommitted={outlineResult.reload}
          />
        ) : null}
        {shapeDialog && outline ? (
          <ShapeDialog
            request={shapeDialog}
            target={{ doc, glyph: shownScene.name, layer: outline.layer }}
            curves={inferCurves(
              outline.contours,
              summary.format,
              outline.curve_format,
            )}
            onClose={() => setShapeDialog(null)}
            onCommitted={outlineResult.reload}
          />
        ) : null}
      </div>
    </div>
  )
}
