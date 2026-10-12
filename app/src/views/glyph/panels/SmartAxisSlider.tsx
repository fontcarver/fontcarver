import { useId, useRef } from 'react'
import type { SmartAxis } from '../../../api'
import styles from './smartComponentControls.module.css'

/** A range gesture writes once on release; Escape restores its starting value. */
export function SmartAxisSlider({
  axis,
  value,
  disabled,
  onPreview,
  onCommit,
  onCancel,
}: {
  axis: SmartAxis
  value: number
  disabled: boolean
  onPreview: (value: number) => void
  onCommit: () => void
  onCancel: () => void
}) {
  const id = useId()
  const pointer = useRef<number | null>(null)
  return (
    <input
      id={id}
      className={styles.slider}
      type="range"
      aria-label={axis.name}
      min={axis.min}
      max={axis.max}
      step="any"
      value={value}
      disabled={disabled}
      onChange={(event) => onPreview(Number(event.currentTarget.value))}
      onPointerDown={(event) => {
        pointer.current = event.pointerId
        event.currentTarget.setPointerCapture?.(event.pointerId)
      }}
      onPointerUp={() => {
        pointer.current = null
        onCommit()
      }}
      onPointerCancel={() => {
        pointer.current = null
        onCancel()
      }}
      onKeyDown={(event) => {
        if (event.key !== 'Escape') return
        event.preventDefault()
        pointer.current = null
        onCancel()
      }}
      onKeyUp={(event) => {
        if (
          [
            'ArrowLeft',
            'ArrowRight',
            'ArrowUp',
            'ArrowDown',
            'Home',
            'End',
            'PageUp',
            'PageDown',
          ].includes(event.key)
        )
          onCommit()
      }}
      onBlur={() => {
        if (pointer.current === null) onCommit()
      }}
    />
  )
}
