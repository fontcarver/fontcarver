import { useEffect, useMemo } from 'react'

import type { DocSummary, GlyphId } from '../api'
import { useFontInfo } from '../stores/fontInfoCache'
import { useEditorStore } from '../stores/editorStore'
import { buildScene } from '../views/glyph/glyphScene'
import { useGlyphDetail } from '../views/glyph/useGlyphDetail'
import { useLayerOutline } from '../views/glyph/useLayerOutline'
import { layerMaster } from '../views/glyph/panels/layerRows'
import { selectDocView, useDocumentStore } from '../stores/documentStore'

/**
 * Publishes the active glyph for a detached Inspector or sidebar.
 *
 * In the main window the glyph canvas publishes it, and the panels read that
 * one record rather than asking for the data again. A detached panel window has
 * no canvas, so without this it would wait for a glyph that nobody was ever
 * going to publish. It therefore runs the same four pieces the canvas runs — the
 * glyph detail, the font info, `buildScene` and the editable layer outline — and
 * publishes the result. It renders nothing: this is the canvas's data path
 * without the canvas.
 */
export function DetachedGlyphSource({
  summary,
  gid,
}: {
  summary: DocSummary
  gid: GlyphId
}) {
  const doc = summary.doc
  const view = useDocumentStore(selectDocView(doc))
  const setActiveGlyph = useEditorStore((state) => state.setActiveGlyph)
  const editedLayer = useEditorStore((state) => state.editedLayer)
  const masterId = view.master ?? summary.masters[0]?.id ?? null

  const detail = useGlyphDetail(doc, gid, masterId)
  const fontInfo = useFontInfo(doc, detail.fontGeneration)

  // A layer picked in this window's Layers panel belongs to its own master.
  const editingMasterId = useMemo(() => {
    if (!editedLayer) return masterId
    const layer = detail.detail?.glyph.layers.find(
      (entry) => entry.id === editedLayer,
    )
    return layer ? (layerMaster(layer.kind) ?? masterId) : masterId
  }, [detail.detail, editedLayer, masterId])

  const master = useMemo(() => {
    const masters = fontInfo.info?.masters ?? []
    return (
      masters.find((entry) => entry.id === (editingMasterId ?? '')) ??
      masters[0] ??
      null
    )
  }, [editingMasterId, fontInfo.info])

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
      editingMasterId,
      master,
      summary.units_per_em,
    ],
  )

  const outlineResult = useLayerOutline(doc, gid, baseScene?.layer?.id ?? null)
  const outline = outlineResult.outline

  // The same merge the canvas performs: the editable outline replaces the
  // display copy as soon as it arrives, so the panels edit the geometry the
  // commands address.
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

  useEffect(() => {
    if (!scene || !detail.detail) {
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
    return () => setActiveGlyph(null)
  }, [
    detail.cell,
    detail.detail,
    doc,
    editingMasterId,
    gid,
    master,
    outline,
    outlineResult.reload,
    scene,
    setActiveGlyph,
  ])

  return null
}
