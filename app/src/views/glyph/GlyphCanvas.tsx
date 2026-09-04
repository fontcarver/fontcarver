import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type RefObject,
} from 'react'

import { Viewport } from '../../canvas/Viewport'
import { registerEditor } from '../../editor/activeEditor'
import {
  EditorController,
  type EditorLabels,
  type EditorSource,
  type PointerInput,
} from '../../editor/editorController'
import type { Modifiers } from '../../editor/gestures/drag'
import { selectActiveTool, useEditorStore } from '../../stores/editorStore'
import type { Selection } from '../../editor/selection'

import type { MetricLabels } from './drawMetrics'
import styles from './glyphView.module.css'
import { sceneBox, type GlyphScene } from './glyphScene'
import type { GlyphTheme } from './glyphTheme'
import { renderGlyph, type GlyphOverlays } from './renderGlyph'

/** Padding kept around the glyph when the view is fitted, in CSS pixels. */
const FIT_PADDING = 48

/** Imperative operations the surrounding view and the commands need. */
export interface GlyphCanvasHandle {
  fit: () => void
  zoomBy: (factor: number) => void
  redraw: () => void
}

export interface GlyphCanvasProps {
  scene: GlyphScene
  theme: GlyphTheme
  labels: MetricLabels
  overlays: GlyphOverlays
  /** Component-flattened outline of the shown layer, in font units. */
  flattened: Path2D | null
  /** Identity of the glyph shown; changing it refits the view. */
  fitKey: string
  label: string
  /** The editable layer and everything the editor needs about it. */
  source: EditorSource | null
  /** Translated undo labels and readout words. */
  editorLabels: EditorLabels
  /** Reports the pointer position in font units, or `null` when it leaves. */
  onPointerFont: (point: { x: number; y: number } | null) => void
  /** Reports the zoom after every viewport change. */
  onZoomChange: (scale: number) => void
  /** Toggles the fill preview when the space bar is tapped. */
  onToggleFilled: () => void
  /** A gesture finished; the outline must be refetched. */
  onReload: () => void
  handleRef?: RefObject<GlyphCanvasHandle | null>
}

function modifiersOf(event: {
  shiftKey: boolean
  altKey: boolean
  ctrlKey: boolean
  metaKey: boolean
}): Modifiers {
  return {
    shift: event.shiftKey,
    alt: event.altKey,
    mod: event.ctrlKey || event.metaKey,
  }
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
  fitKey,
  label,
  source,
  editorLabels,
  onPointerFont,
  onZoomChange,
  onToggleFilled,
  onReload,
  handleRef,
}: GlyphCanvasProps) {
  const wrapperRef = useRef<HTMLDivElement>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const viewport = useMemo(() => new Viewport(), [])
  const pointerRef = useRef<{ x: number; y: number } | null>(null)
  const fittedRef = useRef('')
  const tool = useEditorStore(selectActiveTool)

  const frame = useRef({ scene, theme, labels, overlays, flattened })
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
    const editing = controller.frame()
    renderGlyph(ctx, viewport, {
      scene: editing.contours
        ? {
            ...current.scene,
            contours: editing.contours,
            anchors: editing.anchors ?? current.scene.anchors,
          }
        : current.scene,
      theme: current.theme,
      labels: current.labels,
      overlays: current.overlays,
      flattened: current.flattened,
      pointer: pointerRef.current,
      selected: editing.selected,
      editing: editing.overlay,
      hud: editing.hud,
      pointerScreen: editing.pointerScreen,
    })
  }, [controller, viewport])

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
    })
  }, [controller, draw, onReload, onZoomChange])

  useEffect(
    () => controller.setLabels(editorLabels),
    [controller, editorLabels],
  )
  useEffect(() => controller.setTool(tool), [controller, tool])
  useEffect(() => {
    controller.setSource(source)
    draw()
  }, [controller, draw, source])
  useEffect(() => registerEditor(controller), [controller])
  useEffect(() => () => controller.dispose(), [controller])

  const fit = useCallback(() => {
    viewport.fit(sceneBox(frame.current.scene), FIT_PADDING)
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
    () => ({ fit, zoomBy, redraw: draw }),
    [draw, fit, zoomBy],
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

  // Refresh the draw inputs and repaint whenever anything visible changes.
  useEffect(() => {
    frame.current = { scene, theme, labels, overlays, flattened }
    draw()
  }, [draw, scene, theme, labels, overlays, flattened])

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
      controller.pointerDown(pointerInput(event))
    },
    [controller, pointerInput],
  )

  const onPointerMove = useCallback(
    (event: React.PointerEvent<HTMLCanvasElement>) => {
      const input = pointerInput(event)
      pointerRef.current = viewport.toFont({ x: input.x, y: input.y })
      onPointerFont(roundPoint(pointerRef.current))
      controller.pointerMove(input)
      draw()
    },
    [controller, draw, onPointerFont, pointerInput, viewport],
  )

  const endPointer = useCallback(
    (event: React.PointerEvent<HTMLCanvasElement>) => {
      controller.pointerUp(pointerInput(event))
      try {
        event.currentTarget.releasePointerCapture(event.pointerId)
      } catch {
        // The capture was never established.
      }
    },
    [controller, pointerInput],
  )

  const onPointerLeave = useCallback(() => {
    pointerRef.current = null
    onPointerFont(null)
    controller.pointerLeave()
  }, [controller, onPointerFont])

  const onKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLCanvasElement>) => {
      const modifiers = modifiersOf(event)
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
      if (event.key === 'Escape') {
        event.preventDefault()
        void controller.endNudge()
        controller.escape()
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
    </div>
  )
}

function roundPoint(point: { x: number; y: number }): {
  x: number
  y: number
} {
  return { x: Math.round(point.x), y: Math.round(point.y) }
}
