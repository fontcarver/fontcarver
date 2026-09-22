import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import type { BitmapGlyph } from '../../api'
import { renderBitmap } from '../../canvas/bitmapRenderer'
import { useUiStore } from '../../stores/uiStore'
import {
  paint,
  rectangle,
  type PixelPoint,
  type PixelSelection,
  type PixelTool,
} from './pixels'
import { moveSelection } from './bitmapTransforms'
import styles from './BitmapView.module.css'
interface Props {
  value: BitmapGlyph
  zoom: number
  tool: PixelTool
  ink: number
  disabled: boolean
  selection?: PixelSelection
  onSelect: (selection: PixelSelection) => void
  onEdit: (value: BitmapGlyph) => void
  onCommit: () => void
}
export function BitmapCanvas({
  value,
  zoom,
  tool,
  ink,
  disabled,
  selection,
  onSelect,
  onEdit,
  onCommit,
}: Props) {
  const { t } = useTranslation(),
    ref = useRef<HTMLCanvasElement>(null)
  const gesture = useRef<{
    start: PixelPoint
    last: PixelPoint
    source: BitmapGlyph
    current: BitmapGlyph
    moving?: PixelSelection
  } | null>(null)
  const theme = useUiStore((state) => state.theme)
  const scale = Math.min(
    zoom,
    2048 / Math.max(1, value.bitmap.width, value.bitmap.height),
  )
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
        paper: token('--color-canvas', '#fff'),
        ink: token('--color-text', '#151515'),
        grid: token('--color-border', '#bbb'),
        accent: token('--color-accent', '#547ce5'),
      },
      selection,
    )
  }, [value, scale, selection, theme])
  const point = (event: React.PointerEvent<HTMLCanvasElement>): PixelPoint => {
    const bounds = event.currentTarget.getBoundingClientRect()
    return [
      Math.max(
        0,
        Math.min(
          value.bitmap.width - 1,
          Math.floor(
            ((event.clientX - bounds.left) / bounds.width) * value.bitmap.width,
          ),
        ),
      ),
      Math.max(
        0,
        Math.min(
          value.bitmap.height - 1,
          Math.floor(
            ((event.clientY - bounds.top) / bounds.height) *
              value.bitmap.height,
          ),
        ),
      ),
    ]
  }
  return (
    <div className={styles.canvasArea}>
      <canvas
        ref={ref}
        className={styles.canvas}
        aria-label={t('bitmap.canvas')}
        tabIndex={0}
        onPointerDown={(event) => {
          if (
            disabled ||
            event.button !== 0 ||
            !value.bitmap.width ||
            !value.bitmap.height
          )
            return
          event.preventDefault()
          event.currentTarget.focus()
          event.currentTarget.setPointerCapture(event.pointerId)
          const start = point(event),
            next = paint(value, tool, start, start, ink)
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
            moving,
          }
          if (tool === 'select') {
            if (!moving) onSelect(rectangle(start, start))
          } else onEdit(next)
        }}
        onPointerMove={(event) => {
          const active = gesture.current
          if (!active || disabled) return
          const end = point(event)
          if (active.moving) {
            const dx = end[0] - active.start[0],
              dy = end[1] - active.start[1]
            onEdit(moveSelection(active.source, active.moving, dx, dy))
            onSelect({
              ...active.moving,
              x: active.moving.x + dx,
              y: active.moving.y + dy,
            })
          } else if (tool === 'select') onSelect(rectangle(active.start, end))
          else if (tool !== 'fill') {
            const shape =
              tool === 'line' || tool === 'rectangle' || tool === 'ellipse'
            const next = paint(
              shape ? active.source : active.current,
              tool,
              shape ? active.start : active.last,
              end,
              ink,
            )
            active.current = next
            onEdit(next)
          }
          active.last = end
        }}
        onPointerUp={() => {
          if (!gesture.current) return
          const moved = gesture.current.moving
          gesture.current = null
          if (tool !== 'select' || moved) onCommit()
        }}
        onPointerCancel={() => {
          gesture.current = null
          onCommit()
        }}
        onLostPointerCapture={() => {
          if (gesture.current) {
            gesture.current = null
            onCommit()
          }
        }}
      />
    </div>
  )
}
