import { useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { SmartAxis } from '../../../api'
import { clampSmart, smartAtFraction, smartValue } from './smartComponentValues'
import styles from './smartComponentControls.module.css'

/** Inspector parameter plane for editing two component coordinates together. */
export function SmartComponentHandle({
  axes,
  values,
  disabled,
  onPreview,
  onCommit,
  onCancel,
}: {
  axes: SmartAxis[]
  values: Record<string, number>
  disabled: boolean
  onPreview: (values: Record<string, number>) => void
  onCommit: () => void
  onCancel: () => void
}) {
  const { t } = useTranslation()
  const helpId = useId()
  const [horizontal, setHorizontal] = useState(axes[0]?.name ?? '')
  const [vertical, setVertical] = useState(axes[1]?.name ?? '')
  const pointer = useRef<number | null>(null)
  const handle = useRef<HTMLButtonElement>(null)
  const x = axes.find((axis) => axis.name === horizontal) ?? axes[0]
  const y =
    axes.find((axis) => axis.name === vertical && axis !== x) ??
    axes.find((axis) => axis !== x)
  if (!x || !y) return null
  const xValue = smartValue(x, values)
  const yValue = smartValue(y, values)
  const xPosition = (xValue - x.min) / (x.max - x.min)
  const yPosition = (yValue - y.min) / (y.max - y.min)
  const updatePointer = (
    element: HTMLDivElement,
    clientX: number,
    clientY: number,
  ) => {
    const rect = element.getBoundingClientRect()
    if (!rect.width || !rect.height) return
    onPreview({
      [x.name]: smartAtFraction(x, (clientX - rect.left) / rect.width),
      [y.name]: smartAtFraction(y, 1 - (clientY - rect.top) / rect.height),
    })
  }
  return (
    <div className={styles.handleControl}>
      {axes.length > 2 && (
        <div className={styles.axisSelectors}>
          <label>
            {t('panel.smartComponent.horizontal')}
            <select
              value={x.name}
              disabled={disabled}
              onChange={(event) => setHorizontal(event.target.value)}
            >
              {axes
                .filter((axis) => axis !== y)
                .map((axis) => (
                  <option key={axis.name}>{axis.name}</option>
                ))}
            </select>
          </label>
          <label>
            {t('panel.smartComponent.vertical')}
            <select
              value={y.name}
              disabled={disabled}
              onChange={(event) => setVertical(event.target.value)}
            >
              {axes
                .filter((axis) => axis !== x)
                .map((axis) => (
                  <option key={axis.name}>{axis.name}</option>
                ))}
            </select>
          </label>
        </div>
      )}
      <div
        className={styles.pad}
        role="group"
        aria-label={t('panel.smartComponent.handle')}
        aria-disabled={disabled}
        onPointerDown={(event) => {
          if (disabled || event.button !== 0) return
          event.preventDefault()
          pointer.current = event.pointerId
          event.currentTarget.setPointerCapture?.(event.pointerId)
          handle.current?.focus()
          updatePointer(event.currentTarget, event.clientX, event.clientY)
        }}
        onPointerMove={(event) => {
          if (pointer.current === event.pointerId)
            updatePointer(event.currentTarget, event.clientX, event.clientY)
        }}
        onPointerUp={(event) => {
          if (pointer.current !== event.pointerId) return
          pointer.current = null
          onCommit()
        }}
        onPointerCancel={() => {
          pointer.current = null
          onCancel()
        }}
      >
        <span className={styles.horizontalLabel} aria-hidden>
          {x.name}
        </span>
        <span className={styles.verticalLabel} aria-hidden>
          {y.name}
        </span>
        <button
          ref={handle}
          type="button"
          className={styles.handle}
          style={{
            left: `${xPosition * 100}%`,
            top: `${(1 - yPosition) * 100}%`,
          }}
          disabled={disabled}
          aria-label={t('panel.smartComponent.adjust', {
            x: x.name,
            y: y.name,
          })}
          aria-describedby={helpId}
          onKeyDown={(event) => {
            if (event.key === 'Escape') {
              event.preventDefault()
              pointer.current = null
              onCancel()
              return
            }
            const isX = event.key === 'ArrowLeft' || event.key === 'ArrowRight'
            const isY = event.key === 'ArrowDown' || event.key === 'ArrowUp'
            if (!isX && !isY) return
            event.preventDefault()
            const axis = isX ? x : y
            const sign =
              event.key === 'ArrowLeft' || event.key === 'ArrowDown' ? -1 : 1
            const step =
              ((axis.max - axis.min) / 100) * (event.shiftKey ? 10 : 1)
            onPreview({
              [axis.name]: clampSmart(
                axis,
                smartValue(axis, values) + sign * step,
              ),
            })
          }}
          onKeyUp={(event) => {
            if (event.key.startsWith('Arrow')) onCommit()
          }}
          onBlur={() => {
            if (pointer.current === null) onCommit()
          }}
        />
      </div>
      <p id={helpId} className={styles.hint}>
        {t('panel.smartComponent.handleHint')}
      </p>
    </div>
  )
}
