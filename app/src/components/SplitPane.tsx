import { useCallback, useRef, type ReactNode } from 'react'

import styles from './controls.module.css'

/** Pixels a single arrow-key press moves the divider. */
const KEY_STEP = 8

export interface SplitPaneProps {
  /** Which edge of the workspace the pane is docked to. */
  side: 'left' | 'right'
  size: number
  min: number
  max: number
  /** Accessible name of the divider. */
  label: string
  onResize: (size: number) => void
  children: ReactNode
}

/**
 * A fixed-width pane with a draggable, keyboard-resizable divider. The pane and
 * its divider are siblings in the workspace flex row, ordered by `side`.
 */
export function SplitPane({
  side,
  size,
  min,
  max,
  label,
  onResize,
  children,
}: SplitPaneProps) {
  const dragRef = useRef<{ startX: number; startSize: number } | null>(null)

  const clamp = useCallback(
    (value: number) => Math.min(max, Math.max(min, Math.round(value))),
    [min, max],
  )

  const onPointerDown = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (event.button !== 0) return
      event.currentTarget.setPointerCapture(event.pointerId)
      dragRef.current = { startX: event.clientX, startSize: size }
    },
    [size],
  )

  const onPointerMove = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      const drag = dragRef.current
      if (!drag) return
      const delta = event.clientX - drag.startX
      onResize(clamp(drag.startSize + (side === 'left' ? delta : -delta)))
    },
    [clamp, onResize, side],
  )

  const endDrag = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    if (dragRef.current === null) return
    dragRef.current = null
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId)
  }, [])

  const onKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      const towardsStart = event.key === 'ArrowLeft'
      const towardsEnd = event.key === 'ArrowRight'
      if (!towardsStart && !towardsEnd) return
      event.preventDefault()
      const step =
        (event.shiftKey ? KEY_STEP * 4 : KEY_STEP) * (towardsEnd ? 1 : -1)
      onResize(clamp(size + (side === 'left' ? step : -step)))
    },
    [clamp, onResize, side, size],
  )

  const divider = (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuenow={size}
      aria-valuemin={min}
      aria-valuemax={max}
      tabIndex={0}
      className={styles.gutter}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
      onKeyDown={onKeyDown}
    />
  )

  const pane = (
    <div className={styles.pane} style={{ width: `${size}px` }}>
      {children}
    </div>
  )

  return side === 'left' ? (
    <>
      {pane}
      {divider}
    </>
  ) : (
    <>
      {divider}
      {pane}
    </>
  )
}
