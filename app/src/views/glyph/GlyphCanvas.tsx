import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type RefObject,
} from 'react'

import { recordPaint } from '../../canvas/paintStats'
import { Viewport } from '../../canvas/Viewport'
import { registerEditor } from '../../editor/activeEditor'
import {
  EditorController,
  type EditorLabels,
  type EditorSource,
  type PointerInput,
} from '../../editor/editorController'
import { modifiersOf } from '../../editor/modifiers'
import { reportError } from '../../shell/commands'
import { drawPluginOverlays } from '../../plugins/drawPluginOverlays'
import { useFilterPreview } from '../../plugins/filterSession'
import { usePluginStore } from '../../plugins/pluginStore'
import { selectActiveTool, useEditorStore } from '../../stores/editorStore'
import { useUiStore } from '../../stores/uiStore'
import { usePrefsStore } from '../../stores/prefsStore'
import { useVariationPreviewStore } from '../../stores/variationPreviewStore'
import type { Selection } from '../../editor/selection'
import type { ComponentView } from '../../api'
import type { SnapSettings } from '../../editor/snapping'
import type { ShapeDialogRequest } from '../../editor/tools/toolTypes'
import type { TextCanvasRun } from '../text/textCanvas'
import { TextCanvasPaint } from '../text/textCanvasPaint'
import { drawTextGaps } from '../text/drawTextGaps'
import {
  hitCanvasGap,
  hitCanvasRun,
  localRunBox,
  shiftRunOrigin,
} from '../text/textCanvasBridge'
import { BeamPointer } from './beamPointer'

import { componentDraws, type PathCache } from './componentDraws'
import { withPreview } from './metricsHandles'
import type { MetricLabels } from './drawMetrics'
import type { UnderlayData } from './drawUnderlays'
import styles from './glyphView.module.css'
import { sceneBox, type GlyphScene } from './glyphScene'
import type { GlyphTheme } from './glyphTheme'
import { renderGlyph, type GlyphOverlays } from './renderGlyph'
import {
  useEditorMetadataStore,
  focusedPaths,
} from '../../editor/editorMetadataStore'
import { editorTargetKey } from '../../editor/editorMetadata'
import { useSourceImage } from './useSourceImage'
import { drawPlacement } from '../../canvas/sourceImagePlacement'
import { ImagePlacementPointer } from './panels/imagePlacementPointer'
import { canvasHandlesActive } from './panels/canvasHandleGuard'
import { drawSmartHandles } from './panels/smartComponentCanvas'
import { SmartComponentPointer } from './panels/smartComponentPointer'
import {
  imagePlacementFor,
  useImagePlacement,
} from './panels/imagePlacementSession'
import { OutlineAudit, type OutlineAuditHandle } from './OutlineAudit'

/** Padding kept around the glyph when the view is fitted, in CSS pixels. */
const FIT_PADDING = 48

/** Imperative operations the surrounding view and the commands need. */
export interface GlyphCanvasHandle {
  fit: () => void
  zoomBy: (factor: number) => void
  redraw: () => void
  prepareTransition: () => Promise<void>
}

export interface GlyphCanvasProps {
  scene: GlyphScene
  theme: GlyphTheme
  labels: MetricLabels
  overlays: GlyphOverlays
  /** Component-flattened outline of the shown layer, in font units. */
  flattened: Path2D | null
  /** The other masters and the background layer, drawn behind the outline. */
  underlay: UnderlayData
  /** The layer's components with their own resolved paths. */
  componentViews: readonly ComponentView[]
  /** Identity of the glyph shown; changing it refits the view. */
  fitKey: string
  label: string
  /** The editable layer and everything the editor needs about it. */
  source: EditorSource | null
  /** Translated undo labels and readout words. */
  editorLabels: EditorLabels
  /** Which snap families are live, and how coarse the grid is. */
  snapSettings: SnapSettings
  /** Arrow-key distances from the preferences: plain, Shift, Ctrl. */
  nudgeSteps: readonly [number, number, number]
  /** A shape tool was clicked rather than dragged. */
  onShapeDialog: (request: ShapeDialogRequest) => void
  /** Reports the pointer position in font units, or `null` when it leaves. */
  onPointerFont: (point: { x: number; y: number } | null) => void
  /** Reports the zoom after every viewport change. */
  onZoomChange: (scale: number) => void
  /** Toggles the fill preview when the space bar is tapped. */
  onToggleFilled: () => void
  /** A gesture finished; the outline must be refetched. */
  onReload: () => void
  /** Optional shaped-run shell around the one editable glyph. */
  textRun?: TextCanvasRun | null
  /** A tab-owned metrics session that must close before other edits. */
  externalSettler?: (() => Promise<void>) | null
  handleRef?: RefObject<GlyphCanvasHandle | null>
}

/**
 * The glyph editor canvas.
 *
 * React owns the element, its size and the input events; every pixel is drawn
 * by the plain modules in this folder through `renderGlyph`, and every input
 * decision is made by `EditorController`. The viewport is a mutable object
 * rather than component state, so a pan, a zoom or a drag repaints without a
 * React render.
 */
export function GlyphCanvas({
  scene,
  theme,
  labels,
  overlays,
  flattened,
  underlay,
  componentViews,
  fitKey,
  label,
  source,
  editorLabels,
  snapSettings,
  nudgeSteps,
  onShapeDialog,
  onPointerFont,
  onZoomChange,
  onToggleFilled,
  onReload,
  textRun = null,
  externalSettler = null,
  handleRef,
}: GlyphCanvasProps) {
  const wrapperRef = useRef<HTMLDivElement>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const auditRef = useRef<OutlineAuditHandle>(null)
  const viewport = useMemo(() => new Viewport(), [])
  const pointerRef = useRef<{ x: number; y: number } | null>(null)
  const fittedRef = useRef('')
  const tool = useEditorStore(selectActiveTool)
  const focusState = useEditorMetadataStore((state) => state.focus)
  const activeGlyph = useEditorStore((state) => state.activeGlyph)
  const metadataSource = useMemo(
    () =>
      source
        ? {
            ...source,
            editorMetadata: scene.layer?.user_data,
            glyphLocked:
              activeGlyph?.doc === source.doc &&
              activeGlyph.glyph.name === source.glyph
                ? activeGlyph.glyph.locked
                : false,
          }
        : null,
    [source, scene.layer?.user_data, activeGlyph],
  )
  const pathFocus =
    focusState &&
    source &&
    focusState.owner ===
      editorTargetKey({
        doc: source.doc,
        glyph: source.glyph,
        layer: source.outline.layer,
      })
      ? focusState.contours
      : null
  const snapping = useEditorStore((state) => state.snapping)
  const nodeSize = usePrefsStore((state) => state.prefs.node_size)
  const handleSize = usePrefsStore((state) => state.prefs.handle_size)
  const measurementGuides = useEditorStore((state) => state.measurementGuides)
  const sourceImage = useSourceImage(
    source
      ? { doc: source.doc, glyph: source.glyph, layer: source.outline.layer }
      : null,
    scene.layer?.background_image ?? null,
    scene.layer,
  )
  // Whether the pointer is placing this layer's source image on the canvas.
  const placementRef = useRef(new ImagePlacementPointer())
  const placementOwner = source
    ? editorTargetKey({
        doc: source.doc,
        glyph: source.glyph,
        layer: source.outline.layer,
      })
    : null
  const placementSession = useImagePlacement((store) =>
    store.session?.owner === placementOwner ? store.session : null,
  )
  // Smart-component axis handles, drawn for a selected smart component.
  const smartRef = useRef(new SmartComponentPointer())
  const selection = useEditorStore((state) => state.selection)
  const variation = useVariationPreviewStore((state) =>
    state.preview?.doc === source?.doc && state.preview?.glyph === scene.name
      ? state.preview
      : null,
  )
  const variationPreview = useMemo(
    () =>
      variation
        ? { path: new Path2D(variation.path), width: variation.width }
        : null,
    [variation],
  )

  const frame = useRef({
    scene,
    theme,
    labels,
    overlays,
    flattened,
    underlay,
    sourceImage,
    measurementGuides,
    variationPreview,
    componentViews,
    textRun,
  })
  // Parsing SVG path data is the expensive part of drawing a component, and
  // the strings only change when the kernel answers, so each one is parsed
  // once and reused for every frame of a drag.
  const pathCache = useRef<PathCache>(new Map())
  const textPaint = useMemo(() => new TextCanvasPaint(), [])
  const visibleSignature = useRef('')
  const runOrigin = useRef<{
    fitKey: string
    point: { x: number; y: number }
  } | null>(null)
  const requestedScale = useRef<number | null>(null)
  const [cursor, setCursor] = useState('default')

  // The controller must outlive every render: it holds the live gesture and
  // the selection. Its callbacks, labels and tool are pushed in by effects
  // rather than by rebuilding it.
  const controller = useMemo(
    () => new EditorController(viewport, editorLabels),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [viewport],
  )

  const draw = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const width = Math.max(1, Math.round(viewport.width * viewport.dpr))
    const height = Math.max(1, Math.round(viewport.height * viewport.dpr))
    if (canvas.width !== width) canvas.width = width
    if (canvas.height !== height) canvas.height = height
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const current = frame.current
    const placing = imagePlacementFor(placementOwner) !== null
    const editing = controller.frame()
    const components = componentDraws(
      current.componentViews,
      editing.components,
      pathCache.current,
    )
    const passive = current.textRun
      ? textPaint.prepare(current.textRun, viewport, source === null)
      : null
    // Dev builds time every frame so the 60 fps target of design spec §13 can
    // be measured from the running app rather than asserted.
    const started = import.meta.env.DEV ? performance.now() : 0
    // A live gesture draws its own geometry: the optimistic contours and
    // anchors of a drag, and the metrics a sidebearing drag is showing.
    const scene =
      editing.contours || editing.metrics
        ? {
            ...current.scene,
            contours: editing.contours ?? current.scene.contours,
            anchors: editing.anchors ?? current.scene.anchors,
            metrics: editing.metrics
              ? withPreview(current.scene.metrics, editing.metrics)
              : current.scene.metrics,
          }
        : current.scene
    renderGlyph(ctx, viewport, {
      scene,
      focusedContours: focusedPaths(
        source
          ? {
              doc: source.doc,
              glyph: source.glyph,
              layer: source.outline.layer,
            }
          : null,
      ),
      glyphLocked: metadataSource?.glyphLocked,
      theme: current.theme,
      labels: current.labels,
      overlays: current.overlays,
      flattened: current.flattened,
      underlay: current.underlay,
      sourceImage: placing
        ? placementRef.current.preview(current.sourceImage)
        : current.sourceImage,
      components,
      pointer: pointerRef.current,
      selected: editing.selected,
      editing: editing.overlay,
      tool: editing.tool,
      grid: controller.gridSize,
      nodeSize,
      handleSize,
      hud: editing.hud,
      pointerScreen: editing.pointerScreen,
      hoveredComponent: editing.hoveredComponent,
      measurementGuides: current.measurementGuides,
      variationPreview: current.variationPreview,
      guideOwner: source
        ? {
            doc: source.doc,
            layer: source.outline.layer,
            master: source.master,
          }
        : undefined,
      surrounding: passive?.surrounding,
      hideActive:
        current.textRun !== null &&
        (current.textRun.mode === 'text' || source === null),
    })
    if (current.textRun?.mode !== 'text')
      drawPluginOverlays(ctx, viewport, {
        source,
        scene,
        theme: current.theme,
      })
    if (current.textRun) {
      const visible = passive?.visible ?? []
      drawTextGaps(ctx, viewport, current.textRun, current.theme, visible)
      const signature = visible.join(',')
      if (signature !== visibleSignature.current) {
        visibleSignature.current = signature
        current.textRun.onVisibleIndexes(visible)
      }
    }
    // A view tool owns the pointer, so its handles are not drawn either. A
    // space-held pan is transient and only routes the press, so it does not
    // repaint: reading that ref here would make every key press a paint input.
    const handlesActive = canvasHandlesActive(tool, false)
    const smartHandles = handlesActive
      ? smartRef.current.handles(
          current.componentViews,
          selection,
          viewport,
          metadataSource?.glyphLocked ?? false,
        )
      : []
    drawSmartHandles(
      ctx,
      viewport,
      smartHandles,
      {
        track: current.theme.component,
        handle: current.theme.selection,
        text: current.theme.metricLabel,
        font: current.theme.fontFamily,
      },
      smartRef.current.active(),
    )
    if (placing && handlesActive && current.sourceImage)
      drawPlacement(
        ctx,
        viewport,
        placementRef.current.preview(current.sourceImage) ??
          current.sourceImage,
        {
          frame: current.theme.guideLocal,
          handle: current.theme.selection,
        },
        placementRef.current.handle,
      )
    auditRef.current?.refresh()
    if (import.meta.env.DEV) recordPaint(performance.now() - started)
  }, [
    controller,
    handleSize,
    metadataSource?.glyphLocked,
    nodeSize,
    placementOwner,
    selection,
    source,
    textPaint,
    tool,
    viewport,
  ])

  useEffect(() => {
    const stopPlugins = usePluginStore.subscribe(draw)
    const stopPreview = useFilterPreview.subscribe(draw)
    return () => {
      stopPlugins()
      stopPreview()
    }
  }, [draw])

  useEffect(() => {
    controller.setCallbacks({
      repaint: draw,
      onSelectionChange: (selection: Selection) =>
        useEditorStore.getState().setSelection(selection),
      onEditingChange: (editing) =>
        useEditorStore.getState().setEditing(editing),
      onZoomChange,
      onReload,
      onCursorChange: setCursor,
      onPreview: (preview) => useEditorStore.getState().setPreview(preview),
      onShapeDialog,
      onError: reportError,
      // A lock that does nothing and says nothing cannot be acted on, so a
      // press on one explains itself once; repeats collapse into the one toast.
      onRefused: (reason) => {
        const ui = useUiStore.getState()
        // Pressing a locked path repeatedly should say it once, not stack.
        if (ui.toasts.some((toast) => toast.message === reason)) return
        ui.pushToast({ kind: 'info', message: reason })
      },
    })
  }, [controller, draw, onReload, onShapeDialog, onZoomChange])

  useEffect(
    () => controller.setLabels(editorLabels),
    [controller, editorLabels],
  )
  useEffect(() => controller.setTool(tool), [controller, tool])
  useEffect(() => controller.setSnapping(snapping), [controller, snapping])
  useEffect(
    () => controller.setRulersVisible(overlays.rulers),
    [controller, overlays.rulers],
  )
  useEffect(
    () => controller.setVerticalMode(overlays.verticalMode),
    [controller, overlays.verticalMode],
  )
  useEffect(() => {
    controller.setSnapSettings(snapSettings)
    draw()
  }, [controller, draw, snapSettings])
  useEffect(
    () => controller.setNudgeSteps(nudgeSteps),
    [controller, nudgeSteps],
  )
  useEffect(() => {
    controller.setSource(metadataSource)
    controller.setPathFocus(pathFocus)
    draw()
  }, [controller, draw, metadataSource, pathFocus])
  useEffect(() => {
    const focus = useEditorMetadataStore.getState().focus
    if (
      !source ||
      (focus &&
        focus.owner !==
          editorTargetKey({
            doc: source.doc,
            glyph: source.glyph,
            layer: source.outline.layer,
          }))
    )
      useEditorMetadataStore.getState().clearFocus()
  }, [source])
  useEffect(() => {
    controller.setExternalSettler(externalSettler)
    return () => controller.setExternalSettler(null)
  }, [controller, externalSettler])
  useEffect(() => registerEditor(controller), [controller])
  useEffect(() => () => controller.dispose(), [controller])

  const fit = useCallback(() => {
    const run = frame.current.textRun
    viewport.fit(
      run ? localRunBox(run) : sceneBox(frame.current.scene),
      FIT_PADDING,
    )
    onZoomChange(viewport.scale)
    draw()
  }, [draw, onZoomChange, viewport])

  const zoomBy = useCallback(
    (factor: number) => {
      viewport.zoomAt({ x: viewport.width / 2, y: viewport.height / 2 }, factor)
      onZoomChange(viewport.scale)
      draw()
    },
    [draw, onZoomChange, viewport],
  )

  const handle = useMemo<GlyphCanvasHandle>(
    () => ({
      fit,
      zoomBy,
      redraw: draw,
      prepareTransition: () => controller.prepareTransition(),
    }),
    [controller, draw, fit, zoomBy],
  )

  useEffect(() => {
    if (!handleRef) return
    handleRef.current = handle
    return () => {
      if (handleRef.current === handle) handleRef.current = null
    }
  }, [handle, handleRef])

  // A Glyph tab replaces the previous tab content, so focusing on each fitKey
  // change makes Space-tap and Space-drag available immediately on opening.
  useEffect(() => {
    // Entering text mode also refits the run. Its text field already owns focus.
    if (frame.current.textRun?.mode === 'text') return
    canvasRef.current?.focus({ preventScroll: true })
  }, [fitKey])

  // Size tracking. The first measurement, and every change of glyph, refits.
  useEffect(() => {
    const wrapper = wrapperRef.current
    if (!wrapper) return
    const measure = () => {
      const rect = wrapper.getBoundingClientRect()
      viewport.setSize(
        rect.width,
        rect.height,
        globalThis.devicePixelRatio || 1,
      )
      if (fittedRef.current !== fitKey && rect.width > 0 && rect.height > 0) {
        fittedRef.current = fitKey
        fit()
        return
      }
      draw()
    }
    measure()
    if (typeof ResizeObserver === 'undefined') return
    const observer = new ResizeObserver(measure)
    observer.observe(wrapper)
    return () => observer.disconnect()
  }, [draw, fit, fitKey, viewport])

  useEffect(() => {
    if (!textRun) {
      runOrigin.current = null
      requestedScale.current = null
      visibleSignature.current = ''
      return
    }
    const previous = runOrigin.current
    if (previous?.fitKey === fitKey)
      shiftRunOrigin(viewport, previous.point, textRun.activeOrigin)
    runOrigin.current = { fitKey, point: textRun.activeOrigin }

    if (
      requestedScale.current !== null &&
      requestedScale.current !== textRun.requestedScale
    ) {
      viewport.zoomTo(textRun.requestedScale)
      onZoomChange(viewport.scale)
    }
    requestedScale.current = textRun.requestedScale
    draw()
  }, [draw, fitKey, onZoomChange, textRun, viewport])

  // Refresh inputs before any sizing/origin effect draws the new run.
  useLayoutEffect(() => {
    frame.current = {
      scene,
      theme,
      labels,
      overlays,
      flattened,
      underlay,
      sourceImage,
      measurementGuides,
      variationPreview,
      componentViews,
      textRun,
    }
    draw()
  }, [
    draw,
    scene,
    theme,
    labels,
    overlays,
    flattened,
    underlay,
    sourceImage,
    measurementGuides,
    variationPreview,
    componentViews,
    textRun,
  ])

  // Wheel must be a non-passive native listener: React attaches `onWheel`
  // passively at the root, where `preventDefault` is ignored.
  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const onWheel = (event: WheelEvent) => {
      event.preventDefault()
      const rect = canvas.getBoundingClientRect()
      const at = { x: event.clientX - rect.left, y: event.clientY - rect.top }
      const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? 400 : 1
      if (event.ctrlKey || event.metaKey) {
        controller.zoomAtPoint(at, Math.exp(-event.deltaY * unit * 0.0025))
      } else {
        viewport.pan(-event.deltaX * unit, -event.deltaY * unit)
      }
      pointerRef.current = viewport.toFont(at)
      onPointerFont(roundPoint(pointerRef.current))
      draw()
    }
    canvas.addEventListener('wheel', onWheel, { passive: false })
    return () => canvas.removeEventListener('wheel', onWheel)
  }, [controller, draw, onPointerFont, viewport])

  // Space is both "pan while held" and "toggle the fill preview when tapped".
  const spaceRef = useRef({ held: false, dragged: false })
  // Whether the pointer is dragging the spacing beam rather than the outline.
  const beamRef = useRef(new BeamPointer())

  const pointerInput = useCallback(
    (event: React.PointerEvent<HTMLCanvasElement>): PointerInput => {
      const rect = event.currentTarget.getBoundingClientRect()
      return {
        x: event.clientX - rect.left,
        y: event.clientY - rect.top,
        button: event.button,
        modifiers: modifiersOf(event),
      }
    },
    [],
  )

  const onPointerDown = useCallback(
    (event: React.PointerEvent<HTMLCanvasElement>) => {
      event.preventDefault()
      event.currentTarget.focus({ preventScroll: true })
      spaceRef.current.dragged = spaceRef.current.held
      try {
        event.currentTarget.setPointerCapture(event.pointerId)
      } catch {
        // A synthetic pointer id has nothing to capture; the move still tracks.
      }
      const input = pointerInput(event)
      const run = frame.current.textRun
      if (
        run &&
        input.button === 0 &&
        !spaceRef.current.held &&
        tool !== 'hand' &&
        tool !== 'zoom'
      ) {
        const gap = hitCanvasGap(run, viewport, input)
        if (gap !== null && run.onSelectGap) {
          run.onSelectGap(gap)
          return
        }
        // The beam is a thin band across the whole run, so it is offered after
        // the gap handles and before the glyph under the pointer.
        if (beamRef.current.down(run, viewport, input)) return
        const index = hitCanvasRun(run, viewport, input)
        if (
          index !== null &&
          (run.mode === 'text' || index !== run.activeIndex)
        ) {
          void controller.prepareTransition().then(() => run.onActivate(index))
          return
        }
        if (run.mode === 'text') return
      }
      // Axis and placement handles sit above the outline, so they go first —
      // unless a view gesture is active, which owns the press everywhere.
      const handlesActive = canvasHandlesActive(tool, spaceRef.current.held)
      if (
        handlesActive &&
        smartRef.current.down(
          frame.current.componentViews,
          useEditorStore.getState().selection,
          viewport,
          metadataSource?.glyphLocked ?? false,
          input,
        )
      ) {
        draw()
        return
      }
      if (
        handlesActive &&
        placementRef.current.down(
          viewport,
          frame.current.sourceImage,
          placementSession,
          input,
        )
      ) {
        draw()
        return
      }
      controller.pointerDown(input)
    },
    [
      controller,
      draw,
      metadataSource?.glyphLocked,
      placementSession,
      pointerInput,
      tool,
      viewport,
    ],
  )

  const onPointerMove = useCallback(
    (event: React.PointerEvent<HTMLCanvasElement>) => {
      const input = pointerInput(event)
      pointerRef.current = viewport.toFont({ x: input.x, y: input.y })
      onPointerFont(roundPoint(pointerRef.current))
      if (beamRef.current.move(frame.current.textRun, viewport, input)) return
      const handlesActive = canvasHandlesActive(tool, spaceRef.current.held)
      const overSmart =
        handlesActive &&
        smartRef.current.move(
          frame.current.componentViews,
          useEditorStore.getState().selection,
          viewport,
          metadataSource?.glyphLocked ?? false,
          input,
        )
      if (smartRef.current.busy) {
        if (overSmart) draw()
        return
      }
      if (overSmart) draw()
      if (handlesActive && placementSession) {
        const changed = placementRef.current.move(
          viewport,
          frame.current.sourceImage,
          input,
        )
        const cursor = placementRef.current.cursor()
        if (cursor !== null) setCursor(cursor)
        if (changed) draw()
        if (placementRef.current.busy || cursor !== null) return
      }
      // The controller repaints whatever the move changed; painting again
      // here would double the work of every pointer move.
      controller.pointerMove(input)
    },
    [
      controller,
      draw,
      metadataSource?.glyphLocked,
      onPointerFont,
      placementSession,
      pointerInput,
      tool,
      viewport,
    ],
  )

  const endPointer = useCallback(
    (event: React.PointerEvent<HTMLCanvasElement>) => {
      if (
        smartRef.current.up(
          frame.current.componentViews,
          source
            ? {
                doc: source.doc,
                glyph: source.glyph,
                layer: source.outline.layer,
              }
            : null,
          onReload,
        )
      ) {
        draw()
      } else if (placementRef.current.up(placementSession)) {
        draw()
      } else if (!beamRef.current.up(frame.current.textRun))
        controller.pointerUp(pointerInput(event))
      try {
        event.currentTarget.releasePointerCapture(event.pointerId)
      } catch {
        // The capture was never established.
      }
    },
    [controller, draw, onReload, placementSession, pointerInput, source],
  )

  const onPointerLeave = useCallback(() => {
    pointerRef.current = null
    onPointerFont(null)
    placementRef.current.cancel()
    smartRef.current.cancel()
    controller.pointerLeave()
  }, [controller, onPointerFont])

  const onKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLCanvasElement>) => {
      const modifiers = modifiersOf(event)
      if (['Shift', 'Alt', 'Control', 'Meta'].includes(event.key))
        controller.modifiersChanged(modifiers)
      const run = frame.current.textRun
      if (run?.onKerningKey?.(event)) {
        event.preventDefault()
        event.stopPropagation()
        return
      }
      if (
        run &&
        modifiers.mod &&
        modifiers.alt &&
        (event.key === 'ArrowLeft' || event.key === 'ArrowRight')
      ) {
        event.preventDefault()
        return
      }
      if (run?.onMetricKey?.(event)) {
        event.preventDefault()
        event.stopPropagation()
        return
      }
      if (run?.mode === 'text' && event.key !== ' ' && event.key !== 'Escape')
        return
      if (event.key === ' ' && !modifiers.mod && !modifiers.alt) {
        event.preventDefault()
        if (event.repeat) return
        spaceRef.current = { held: true, dragged: false }
        controller.setSpaceHeld(true)
        return
      }
      const arrows: Record<string, { x: number; y: number }> = {
        ArrowLeft: { x: -1, y: 0 },
        ArrowRight: { x: 1, y: 0 },
        ArrowUp: { x: 0, y: 1 },
        ArrowDown: { x: 0, y: -1 },
      }
      const direction = arrows[event.key]
      if (direction) {
        event.preventDefault()
        controller.nudgeSelection(direction, modifiers)
        return
      }
      // The active tool is offered the key first: Enter finishes a pen path
      // and Escape abandons one, and neither should reach the editor's own
      // Escape or the command registry while a path is being drawn.
      if (
        controller.toolKey({
          key: event.key,
          modifiers,
          repeat: event.repeat,
        })
      ) {
        event.preventDefault()
        event.stopPropagation()
        return
      }
      if (event.key === 'Escape') {
        event.preventDefault()
        if (!controller.escape()) frame.current.textRun?.onExitEdit()
        return
      }
      // Tab cycles nodes only while something is selected; with an empty
      // selection it must still move focus out of the canvas (§12).
      if (event.key === 'Tab' && controller.currentSelection.size > 0) {
        event.preventDefault()
        controller.cycleNodes(modifiers.shift ? -1 : 1)
      }
      // Delete, ⇧Delete, Enter and Mod+A are registered commands, so the
      // shortcut layer owns them and the menu lists them; handling them here
      // as well would run each action twice.
    },
    [controller],
  )

  const onKeyUp = useCallback(
    (event: React.KeyboardEvent<HTMLCanvasElement>) => {
      if (['Shift', 'Alt', 'Control', 'Meta'].includes(event.key))
        controller.modifiersChanged(modifiersOf(event))
      if (event.key !== ' ') return
      event.preventDefault()
      const { held, dragged } = spaceRef.current
      spaceRef.current = { held: false, dragged: false }
      controller.setSpaceHeld(false)
      if (held && !dragged) onToggleFilled()
    },
    [controller, onToggleFilled],
  )

  return (
    <div ref={wrapperRef} className={styles.canvasWrapper}>
      <canvas
        ref={canvasRef}
        className={styles.canvas}
        style={{ cursor }}
        role="img"
        aria-label={label}
        tabIndex={0}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endPointer}
        onPointerCancel={endPointer}
        onPointerLeave={onPointerLeave}
        onDoubleClick={(event) => {
          const rect = event.currentTarget.getBoundingClientRect()
          controller.doubleClick({
            x: event.clientX - rect.left,
            y: event.clientY - rect.top,
            button: event.button,
            modifiers: modifiersOf(event),
          })
        }}
        onKeyDown={onKeyDown}
        onKeyUp={onKeyUp}
      />
      {metadataSource && textRun?.mode !== 'text' && (
        <OutlineAudit
          source={metadataSource}
          viewport={viewport}
          onReload={onReload}
          handleRef={auditRef}
        />
      )}
    </div>
  )
}

function roundPoint(point: { x: number; y: number }): {
  x: number
  y: number
} {
  return { x: Math.round(point.x), y: Math.round(point.y) }
}
