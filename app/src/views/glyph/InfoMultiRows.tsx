import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { NumberField } from '../../components/NumberField'
import { boundsOf, pointAt } from '../../editor/outlineGeometry'
import { transformSelectionCommand } from '../../editor/pointCommands'
import {
  selectedIndexes,
  selectedPointRefs,
  selectionCounts,
} from '../../editor/selection'

import { commitField } from './commitField'
import styles from './infoBox.module.css'
import type { InfoRowsProps } from './infoBoxProps'

/** The nine handles of the origin picker, row-major from the top left. */
const ORIGINS: readonly [number, number][] = [
  [0, 1],
  [0.5, 1],
  [1, 1],
  [0, 0.5],
  [0.5, 0.5],
  [1, 0.5],
  [0, 0],
  [0.5, 0],
  [1, 0],
]

/**
 * A multi-selection: its bounding box, a 9-point origin, a proportional lock
 * and the point count (design spec §6.1).
 *
 * Editing X/Y translates the selection and editing W/H scales it about the
 * chosen origin — one `TransformSelection`, one undo step.
 */
export function InfoMultiRows({
  target,
  outline,
  selection,
  onCommitted,
}: InfoRowsProps) {
  const { t } = useTranslation()
  const [origin, setOrigin] = useState(4)
  const [locked, setLocked] = useState(false)

  const points = selectedPointRefs(selection)
  const anchors = selectedIndexes(selection, 'anchor')
  const components = selectedIndexes(selection, 'component')
  const counts = selectionCounts(selection)
  const total = outline.contours.reduce(
    (sum, contour) => sum + contour.points.length,
    0,
  )

  // Everything the transform will move counts towards the box, or the numbers
  // would describe a different selection from the one that is about to change.
  const positions: { x: number; y: number }[] = points
    .map((ref) => pointAt(outline.contours, ref))
    .filter((point): point is NonNullable<typeof point> => point !== null)
  for (const index of anchors) {
    const anchor = outline.anchors[index]
    if (anchor) positions.push({ x: anchor.x, y: anchor.y })
  }
  for (const index of components) {
    const bbox = outline.components[index]?.bbox
    if (!bbox) continue
    positions.push({ x: bbox[0], y: bbox[1] }, { x: bbox[2], y: bbox[3] })
  }
  const box = boundsOf(positions)
  if (!box) return null
  const width = box.x1 - box.x0
  const height = box.y1 - box.y0
  const handle = ORIGINS[origin] ?? [0.5, 0.5]
  const pivot = {
    x: box.x0 + width * handle[0],
    y: box.y0 + height * handle[1],
  }

  const apply = (
    affine: [number, number, number, number, number, number],
    label: string,
  ) =>
    commitField(
      target,
      transformSelectionCommand(
        target,
        { points, components, anchors },
        affine,
      ),
      label,
      onCommitted,
    )

  const translate = (dx: number, dy: number) =>
    apply([1, 0, 0, 1, dx, dy], t('editor.undo.moveSelection'))

  const scale = (sx: number, sy: number) =>
    // Scale about the chosen origin: translate to it, scale, translate back.
    apply(
      [sx, 0, 0, sy, pivot.x - sx * pivot.x, pivot.y - sy * pivot.y],
      t('editor.undo.transform'),
    )

  return (
    <>
      <NumberField
        label={t('infoBox.x')}
        value={box.x0}
        invalidHint={t('infoBox.invalid')}
        onCommit={(next) => translate(next - box.x0, 0)}
      />
      <NumberField
        label={t('infoBox.y')}
        value={box.y0}
        invalidHint={t('infoBox.invalid')}
        onCommit={(next) => translate(0, next - box.y0)}
      />
      <NumberField
        label={t('infoBox.width')}
        value={width}
        disabled={width === 0}
        invalidHint={t('infoBox.invalid')}
        onCommit={(next) => {
          const sx = next / width
          scale(sx, locked ? sx : 1)
        }}
      />
      <NumberField
        label={t('infoBox.height')}
        value={height}
        disabled={height === 0}
        invalidHint={t('infoBox.invalid')}
        onCommit={(next) => {
          const sy = next / height
          scale(locked ? sy : 1, sy)
        }}
      />
      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('infoBox.origin')}</span>
        <span
          className={styles.origin}
          role="radiogroup"
          aria-label={t('infoBox.origin')}
        >
          {ORIGINS.map((_, index) => (
            <button
              key={index}
              type="button"
              role="radio"
              aria-checked={origin === index}
              aria-label={t(`infoBox.originPoint.${index}`)}
              className={
                origin === index
                  ? `${styles.originDot} ${styles.originDotActive}`
                  : styles.originDot
              }
              onClick={() => setOrigin(index)}
            />
          ))}
        </span>
      </div>
      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('infoBox.proportional')}</span>
        <button
          type="button"
          className={styles.toggle}
          aria-pressed={locked}
          onClick={() => setLocked((value) => !value)}
        >
          {locked ? t('infoBox.locked') : t('infoBox.unlocked')}
        </button>
      </div>
      <p className={styles.count}>
        {t('infoBox.count', { selected: counts.nodes + counts.handles, total })}
      </p>
    </>
  )
}
