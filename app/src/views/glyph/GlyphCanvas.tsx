import { useCallback, useEffect, useMemo, useRef, type RefObject } from 'react'

import { Viewport } from '../../canvas/Viewport'

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
  /** Reports the pointer position in font units, or `null` when it leaves. */
  onPointerFont: (point: { x: number; y: number } | null) => void
  /** Reports the zoom after every viewport change. */
  onZoomChange: (scale: number) => void
  /** Toggles the fill preview when the space bar is tapped. */
  onToggleFilled: () => void
  handleRef?: RefObject<GlyphCanvasHandle | null>
}

/**
 * The glyph editor canvas.
 *
 * React owns the element, its size and the input events; every pixel is drawn
 * by the plain modules in this folder through `renderGlyph`. The viewport is a
 * mutable object rather than component state, so a pan or a zoom repaints
 * without a React render.
 */
export function GlyphCanvas({
  scene,
  theme,
  labels,
  overlays,
  flattened,
  fitKey,
  label,
  onPointerFont,
  onZoomChange,
  onToggleFilled,
  handleRef,
}: GlyphCanvasProps) {
  const wrapperRef = useRef<HTMLDivElement>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const viewport = useMemo(() => new Viewport(), [])
  const pointerRef = useRef<{ x: number; y: number } | null>(null)
  const fittedRef = useRef('')

  // The draw inputs are read through a ref so the event handlers installed on
  // the element stay stable while still painting the current scene. The ref is
  // seeded on the first render and refreshed in an effect afterwards.
  const frame = useRef({ scene, theme, labels, overlays, flattened })

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
    renderGlyph(ctx, viewport, {
      scene: current.scene,
      theme: current.theme,
      labels: current.labels,
      overlays: current.overlays,
      flattened: current.flattened,
      pointer: pointerRef.current,
    })
  }, [viewport])

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
        viewport.zoomAt(at, Math.exp(-event.deltaY * unit * 0.0025))
        onZoomChange(viewport.scale)
      } else {
        viewport.pan(-event.deltaX * unit, -event.deltaY * unit)
      }
      pointerRef.current = viewport.toFont(at)
      onPointerFont(roundPoint(pointerRef.current))
      draw()
    }
    canvas.addEventListener('wheel', onWheel, { passive: false })
    return () => canvas.removeEventListener('wheel', onWheel)
  }, [draw, onPointerFont, onZoomChange, viewport])

  // Space is both "pan while held" and "toggle the fill preview when tapped".
  const spaceRef = useRef({ held: false, dragged: false })
  const dragRef = useRef<{ id: number; x: number; y: number } | null>(null)

  const onPointerDown = useCallback(
    (event: React.PointerEvent<HTMLCanvasElement>) => {
      const panning =
        event.button === 1 || (event.button === 0 && spaceRef.current.held)
      if (!panning) {
        event.currentTarget.focus()
        return
      }
      event.preventDefault()
      event.currentTarget.focus()
      dragRef.current = {
        id: event.pointerId,
        x: event.clientX,
        y: event.clientY,
      }
      spaceRef.current.dragged = true
      try {
        event.currentTarget.setPointerCapture(event.pointerId)
      } catch {
        // A synthetic pointer id has nothing to capture; the move still tracks.
      }
    },
    [],
  )

  const onPointerMove = useCallback(
    (event: React.PointerEvent<HTMLCanvasElement>) => {
      const rect = event.currentTarget.getBoundingClientRect()
      const at = { x: event.clientX - rect.left, y: event.clientY - rect.top }
      const drag = dragRef.current
      if (drag && drag.id === event.pointerId) {
        viewport.pan(event.clientX - drag.x, event.clientY - drag.y)
        dragRef.current = { id: drag.id, x: event.clientX, y: event.clientY }
      }
      pointerRef.current = viewport.toFont(at)
      onPointerFont(roundPoint(pointerRef.current))
      draw()
    },
    [draw, onPointerFont, viewport],
  )

  const endDrag = useCallback(
    (event: React.PointerEvent<HTMLCanvasElement>) => {
      if (dragRef.current?.id !== event.pointerId) return
      dragRef.current = null
      try {
        event.currentTarget.releasePointerCapture(event.pointerId)
      } catch {
        // The capture was never established.
      }
    },
    [],
  )

  const onPointerLeave = useCallback(() => {
    pointerRef.current = null
    onPointerFont(null)
    draw()
  }, [draw, onPointerFont])

  const onKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLCanvasElement>) => {
      if (event.key !== ' ' || event.ctrlKey || event.metaKey || event.altKey)
        return
      event.preventDefault()
      if (event.repeat) return
      spaceRef.current = { held: true, dragged: false }
    },
    [],
  )

  const onKeyUp = useCallback(
    (event: React.KeyboardEvent<HTMLCanvasElement>) => {
      if (event.key !== ' ') return
      event.preventDefault()
      const { held, dragged } = spaceRef.current
      spaceRef.current = { held: false, dragged: false }
      if (held && !dragged) onToggleFilled()
    },
    [onToggleFilled],
  )

  return (
    <div ref={wrapperRef} className={styles.canvasWrapper}>
      <canvas
        ref={canvasRef}
        className={styles.canvas}
        role="img"
        aria-label={label}
        tabIndex={0}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        onPointerLeave={onPointerLeave}
        onClick={(event) => event.currentTarget.focus({ preventScroll: true })}
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
