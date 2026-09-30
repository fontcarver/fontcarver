import { useTranslation } from 'react-i18next'

import type { Component, ComponentView } from '../../../api'
import { NumberField } from '../../../components/NumberField'

import { SmartComponentControls } from './SmartComponentControls'
import styles from './panels.module.css'

export interface ComponentRowProps {
  view: ComponentView
  index: number
  selected: boolean
  /** Whether a drag is currently carrying this row. */
  dragging: boolean
  onSelect: () => void
  onOpenBase: () => void
  onWrite: (component: Component) => void
  onRemove: () => void
  onDecompose: () => void
  onMove: (to: number) => void
  onDragStart: () => void
  onDragEnd: () => void
  count: number
}

/** Scale percentages read off a component transform's column lengths. */
function scaleOf(transform: readonly number[]): { x: number; y: number } {
  const [a = 1, b = 0, c = 0, d = 1] = transform
  return { x: Math.hypot(a, b) * 100, y: Math.hypot(c, d) * 100 }
}

/** A transform with its scale replaced, keeping its rotation and offset. */
function withScale(
  transform: readonly number[],
  x: number,
  y: number,
): [number, number, number, number, number, number] {
  const [a = 1, b = 0, c = 0, d = 1, e = 0, f = 0] = transform
  const current = scaleOf(transform)
  const sx = current.x === 0 ? 0 : x / current.x
  const sy = current.y === 0 ? 0 : y / current.y
  return [a * sx, b * sx, c * sy, d * sy, e, f]
}

/**
 * One row of the Components section (design spec §7.4): the base glyph, its
 * offset and scale, the two alignment flags the model carries, and the row's
 * own actions. Every control writes one `SetComponent`.
 */
export function ComponentRow({
  view,
  index,
  selected,
  dragging,
  onSelect,
  onOpenBase,
  onWrite,
  onRemove,
  onDecompose,
  onMove,
  onDragStart,
  onDragEnd,
  count,
}: ComponentRowProps) {
  const { t } = useTranslation()
  const component = view.component
  const scale = scaleOf(component.transform)
  const offsetX = component.transform[4] ?? 0
  const offsetY = component.transform[5] ?? 0

  const write = (next: Partial<Component>) => onWrite({ ...component, ...next })

  const move = (transform: [number, number, number, number, number, number]) =>
    onWrite({ ...component, transform })

  return (
    <div
      className={`${styles.listRow} ${selected ? styles.listRowActive : ''} ${
        dragging ? styles.listRowDragging : ''
      }`}
      draggable={!component.locked}
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      onDragOver={(event) => event.preventDefault()}
      onDrop={(event) => {
        event.preventDefault()
        if (!component.locked) onMove(index)
      }}
    >
      <button
        type="button"
        className={styles.rowName}
        aria-current={selected}
        onClick={onSelect}
        onDoubleClick={onOpenBase}
        title={component.base}
      >
        {component.base}
      </button>
      <span className={styles.rowMeta}>
        {`${Math.round(offsetX)}, ${Math.round(offsetY)} · ${Math.round(
          scale.x,
        )}%`}
      </span>
      <button
        type="button"
        className={styles.chipRemove}
        aria-label={t('panel.components.moveUp', { name: component.base })}
        disabled={component.locked || index === 0}
        onClick={() => onMove(index - 1)}
      >
        ↑
      </button>
      <button
        type="button"
        className={styles.chipRemove}
        aria-label={t('panel.components.moveDown', { name: component.base })}
        disabled={component.locked || index + 1 >= count}
        onClick={() => onMove(index + 1)}
      >
        ↓
      </button>
      <button
        type="button"
        className={styles.chipRemove}
        aria-label={t('panel.components.openBase', { name: component.base })}
        onClick={onOpenBase}
      >
        ↗
      </button>
      <button
        type="button"
        className={styles.chipRemove}
        aria-label={t('panel.components.decompose', { name: component.base })}
        disabled={component.locked}
        onClick={onDecompose}
      >
        ⊘
      </button>
      <button
        type="button"
        className={styles.chipRemove}
        aria-label={t('panel.components.remove', { name: component.base })}
        disabled={component.locked}
        onClick={onRemove}
      >
        ×
      </button>
      <button
        type="button"
        className={styles.toggle}
        aria-pressed={component.locked ?? false}
        onClick={() => write({ locked: !component.locked })}
      >
        {t('panel.components.lock')}
      </button>
      {selected ? (
        <div className={styles.rowFields}>
          <SmartComponentControls
            axes={view.smart_axes ?? []}
            component={component}
            disabled={component.locked ?? false}
            onWrite={onWrite}
          />
          <NumberField
            disabled={component.locked}
            label={t('infoBox.x')}
            value={offsetX}
            invalidHint={t('infoBox.invalid')}
            onCommit={(x) => {
              const next = [...component.transform] as [
                number,
                number,
                number,
                number,
                number,
                number,
              ]
              next[4] = x
              move(next)
            }}
          />
          <NumberField
            disabled={component.locked}
            label={t('infoBox.y')}
            value={offsetY}
            invalidHint={t('infoBox.invalid')}
            onCommit={(y) => {
              const next = [...component.transform] as [
                number,
                number,
                number,
                number,
                number,
                number,
              ]
              next[5] = y
              move(next)
            }}
          />
          <NumberField
            disabled={component.locked}
            label={t('infoBox.scaleX')}
            value={scale.x}
            invalidHint={t('infoBox.invalid')}
            onCommit={(x) => move(withScale(component.transform, x, scale.y))}
          />
          <NumberField
            disabled={component.locked}
            label={t('infoBox.scaleY')}
            value={scale.y}
            invalidHint={t('infoBox.invalid')}
            onCommit={(y) => move(withScale(component.transform, scale.x, y))}
          />
          <button
            type="button"
            className={styles.toggle}
            disabled={component.locked}
            aria-pressed={component.auto_align}
            onClick={() => write({ auto_align: !component.auto_align })}
          >
            {t('infoBox.autoAlign')}
          </button>
          <button
            type="button"
            className={styles.toggle}
            disabled={component.locked}
            aria-pressed={component.use_my_metrics}
            onClick={() => write({ use_my_metrics: !component.use_my_metrics })}
          >
            {t('infoBox.useMyMetrics')}
          </button>
        </div>
      ) : null}
    </div>
  )
}
