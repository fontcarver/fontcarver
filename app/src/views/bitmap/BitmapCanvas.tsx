import { useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { BitmapGlyph } from '../../api'
import { renderBitmap } from '../../canvas/bitmapRenderer'
import {
  bitmapFrame,
  bitmapPointer,
  type BitmapFrame,
} from '../../canvas/bitmapGeometry'
import { useUiStore } from '../../stores/uiStore'
import {
  paint,
  pencilInk,
  rectangle,
  sample,
  type PixelPoint,
  type PixelSelection,
  type PixelTool,
} from './pixels'
import { copySelection, moveSelection } from './bitmapTransforms'
import { useBitmapToolStore } from './bitmapToolStore'
import { BitmapCanvasControls } from './BitmapCanvasControls'
import styles from './BitmapCanvas.module.css'

interface Props {
  value: BitmapGlyph
  zoom: number
  tool: PixelTool
  ink: number
  disabled: boolean
  selection?: PixelSelection
  upm?: number
  ppem?: number
  outlinePath?: string
  resolved?: BitmapGlyph | null
  onPickInk?: (ink: number) => void
  onSelect: (selection: PixelSelection) => void
  onEdit: (value: BitmapGlyph) => void
  onCommit: () => void
}
interface Gesture {
  start: PixelPoint
  last: PixelPoint
  source: BitmapGlyph
  current: BitmapGlyph
  tool: PixelTool
  ink: number
  moving?: PixelSelection
  /** Alt-drag leaves the original pixels in place and stamps a copy. */
  stamping?: boolean
  pan?: { x: number; y: number; left: number; top: number }
}

/** Pixel editing surface with font-space guides and immutable gesture sources. */
export function BitmapCanvas({
  value,
  zoom,
  tool,
  ink,
  disabled,
  selection,
  upm,
  ppem,
  outlinePath,
  resolved,
  onPickInk,
  onSelect,
  onEdit,
  onCommit,
}: Props) {
  const { t } = useTranslation()
  const ref = useRef<HTMLCanvasElement>(null)
  const area = useRef<HTMLDivElement>(null)
  const gesture = useRef<Gesture | null>(null)
  const [activeFrame, setActiveFrame] = useState<BitmapFrame | null>(null)
  const overlays = useBitmapToolStore((state) => state.overlays)
  const setOverlay = useBitmapToolStore((state) => state.setOverlay)
  const { grid, outline, advanceColumn, fontUnits } = overlays
  const [pointer, setPointer] = useState<PixelPoint | null>(null)
  const [delta, setDelta] = useState<PixelPoint | null>(null)
  const theme = useUiStore((state) => state.theme)
  const frame = useMemo(
    () =>
      activeFrame ?? bitmapFrame(value.bitmap, value.advance, resolved?.bitmap),
    [activeFrame, value.bitmap, value.advance, resolved],
  )
  const scale = Math.min(zoom, 2048 / Math.max(1, frame.width, frame.height))
  const unitsPerPixel =
    upm && ppem && upm > 0 && ppem > 0 ? upm / ppem : undefined
  useEffect(() => {
    const canvas = ref.current
    if (!canvas) return
    const style = getComputedStyle(canvas)
    const token = (key: string, fallback: string) =>
      style.getPropertyValue(key).trim() || fallback
    renderBitmap(
      canvas,
      value.bitmap,
      scale,
      {
        paper: token('--bm-paper', '#fdfdfb'),
        ink: token('--bm-ink', '#101217'),
        grid: token('--bm-grid', '#c8ccd4'),
        accent: token('--bm-accent', '#2b5fa8'),
      },
      selection,
      true,
      {
        frame,
        grid,
        advance: value.advance,
        advanceColumn,
        outlinePath: outline ? outlinePath : undefined,
        unitsPerPixel,
        resolved: resolved?.bitmap,
      },
    )
  }, [
    value,
    resolved,
    scale,
    selection,
    theme,
    grid,
    outline,
    advanceColumn,
    outlinePath,
    unitsPerPixel,
    frame,
  ])
  const point = (event: React.PointerEvent<HTMLCanvasElement>): PixelPoint => {
    const bounds = event.currentTarget.getBoundingClientRect()
    if (!bounds.width || !bounds.height) return [-1, -1]
    return bitmapPointer(
      value.bitmap,
      frame,
      ((event.clientX - bounds.left) / bounds.width) * frame.width,
      ((event.clientY - bounds.top) / bounds.height) * frame.height,
    )
  }
  const clamp = ([x, y]: PixelPoint): PixelPoint => [
    Math.max(0, Math.min(value.bitmap.width - 1, x)),
    Math.max(0, Math.min(value.bitmap.height - 1, y)),
  ]
  const showPointer = (position: PixelPoint) => {
    setPointer([
      value.bitmap.origin_x + position[0],
      value.bitmap.origin_y + value.bitmap.height - 1 - position[1],
    ])
  }
  const finish = () => {
    const active = gesture.current
    gesture.current = null
    setActiveFrame(null)
    if (
      active &&
      active.tool !== 'hand' &&
      (active.tool !== 'select' || active.moving)
    )
      onCommit()
  }
  return (
    <>
      <BitmapCanvasControls
        overlays={overlays}
        onOverlay={setOverlay}
        hasOutline={Boolean(outlinePath && unitsPerPixel)}
        unitsPerPixel={fontUnits ? unitsPerPixel : undefined}
        unitsPerPixelLabel={unitsPerPixel}
        pointer={pointer}
        delta={delta}
        advance={value.advance}
      />
      <div ref={area} className={styles.area}>
        <canvas
          ref={ref}
          className={styles.canvas}
          data-tool={tool}
          aria-label={t('bitmap.canvas')}
          tabIndex={0}
          onKeyDown={(event) => {
            if (tool !== 'hand' || !area.current) return
            const moves: Record<string, PixelPoint> = {
              ArrowLeft: [-40, 0],
              ArrowRight: [40, 0],
              ArrowUp: [0, -40],
              ArrowDown: [0, 40],
            }
            const move = moves[event.key]
            if (!move) return
            event.preventDefault()
            event.stopPropagation()
            area.current.scrollLeft += move[0]
            area.current.scrollTop += move[1]
          }}
          onPointerDown={(event) => {
            if (
              event.button !== 0 ||
              (disabled && tool !== 'hand' && tool !== 'eyedropper')
            )
              return
            event.preventDefault()
            event.currentTarget.focus()
            const start = point(event)
            showPointer(start)
            setDelta(null)
            if (tool === 'hand' && area.current) {
              event.currentTarget.setPointerCapture(event.pointerId)
              setActiveFrame(frame)
              gesture.current = {
                start,
                last: start,
                source: value,
                current: value,
                tool,
                ink,
                pan: {
                  x: event.clientX,
                  y: event.clientY,
                  left: area.current.scrollLeft,
                  top: area.current.scrollTop,
                },
              }
              return
            }
            if (tool === 'eyedropper') {
              const b = resolved?.bitmap ?? value.bitmap
              onPickInk?.(
                sample(
                  b,
                  start[0] + value.bitmap.origin_x - b.origin_x,
                  start[1] +
                    b.origin_y +
                    b.height -
                    value.bitmap.origin_y -
                    value.bitmap.height,
                ),
              )
              return
            }
            if (
              start[0] < 0 ||
              start[1] < 0 ||
              start[0] >= value.bitmap.width ||
              start[1] >= value.bitmap.height
            )
              return
            event.currentTarget.setPointerCapture(event.pointerId)
            setActiveFrame(frame)
            const gestureInk =
              tool === 'pencil' ? pencilInk(value.bitmap, start, ink) : ink
            const next = paint(value, tool, start, start, gestureInk)
            const moving =
              tool === 'select' &&
              selection &&
              start[0] >= selection.x &&
              start[0] < selection.x + selection.width &&
              start[1] >= selection.y &&
              start[1] < selection.y + selection.height
                ? selection
                : undefined
            gesture.current = {
              start,
              last: start,
              source: value,
              current: next,
              tool,
              ink: gestureInk,
              moving,
              stamping: Boolean(moving) && event.altKey,
            }
            if (tool === 'select') {
              if (!moving) onSelect(rectangle(start, start))
            } else onEdit(next)
          }}
          onPointerMove={(event) => {
            const active = gesture.current
            const raw = point(event)
            showPointer(raw)
            if (!active) return
            if (active.pan && area.current) {
              area.current.scrollLeft =
                active.pan.left + active.pan.x - event.clientX
              area.current.scrollTop =
                active.pan.top + active.pan.y - event.clientY
              return
            }
            if (disabled) return
            const end = clamp(raw)
            setDelta([end[0] - active.start[0], active.start[1] - end[1]])
            if (active.moving) {
              const dx = end[0] - active.start[0],
                dy = end[1] - active.start[1]
              onEdit(
                active.stamping
                  ? copySelection(active.source, active.moving, dx, dy)
                  : moveSelection(active.source, active.moving, dx, dy),
              )
              onSelect({
                ...active.moving,
                x: active.moving.x + dx,
                y: active.moving.y + dy,
              })
            } else if (active.tool === 'select')
              onSelect(rectangle(active.start, end))
            else if (active.tool !== 'fill') {
              const shape =
                active.tool === 'line' ||
                active.tool === 'rectangle' ||
                active.tool === 'ellipse'
              const next = paint(
                shape ? active.source : active.current,
                active.tool,
                shape ? active.start : active.last,
                end,
                active.ink,
              )
              active.current = next
              onEdit(next)
            }
            active.last = end
          }}
          onPointerLeave={() => {
            if (!gesture.current) setPointer(null)
          }}
          onPointerUp={finish}
          onPointerCancel={finish}
          onLostPointerCapture={finish}
        />
      </div>
    </>
  )
}
