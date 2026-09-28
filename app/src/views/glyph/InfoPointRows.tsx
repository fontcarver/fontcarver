import { useTranslation } from 'react-i18next'

import { NumberField } from '../../components/NumberField'
import { handlePolar } from '../../editor/hud'
import { nodeOfHandle, pointAt } from '../../editor/outlineGeometry'
import {
  movePointsCommand,
  nextSmoothFlag,
  setPositionCommand,
  smoothCommand,
} from '../../editor/pointCommands'
import { onlyTarget } from '../../editor/selection'

import { commitField } from './commitField'
import { SmartCornerField } from './SmartCornerField'
import styles from './infoBox.module.css'
import type { InfoRowsProps } from './infoBoxProps'

/**
 * One node or one handle (design spec §6.1).
 *
 * A node edits its absolute position and its smooth flag; a handle edits its
 * offset from the node it belongs to, in cartesian or polar form, because that
 * is how a curve is reasoned about. Every field is one `doc.apply`, i.e. one
 * undo step.
 */
export function InfoPointRows({
  target,
  outline,
  selection,
  onCommitted,
}: InfoRowsProps) {
  const { t } = useTranslation()
  const only = onlyTarget(selection)
  if (!only || (only.kind !== 'node' && only.kind !== 'handle')) return null
  const ref = { contour: only.contour, point: only.point }
  const point = pointAt(outline.contours, ref)
  if (!point) return null

  if (only.kind === 'node')
    return (
      <>
        <NumberField
          label={t('infoBox.x')}
          value={point.x}
          invalidHint={t('infoBox.invalid')}
          onCommit={(x) =>
            commitField(
              target,
              setPositionCommand(target, ref, x, point.y),
              t('editor.undo.setPosition'),
              onCommitted,
            )
          }
        />
        <NumberField
          label={t('infoBox.y')}
          value={point.y}
          invalidHint={t('infoBox.invalid')}
          onCommit={(y) =>
            commitField(
              target,
              setPositionCommand(target, ref, point.x, y),
              t('editor.undo.setPosition'),
              onCommitted,
            )
          }
        />
        <div className={styles.row}>
          <span className={styles.rowLabel}>{t('infoBox.type')}</span>
          <button
            type="button"
            className={styles.toggle}
            aria-pressed={point.smooth}
            onClick={() =>
              commitField(
                target,
                smoothCommand(
                  target,
                  [ref],
                  nextSmoothFlag(outline.contours, [ref]),
                  true,
                ),
                t('editor.undo.toggleSmooth'),
                onCommitted,
              )
            }
          >
            {point.smooth ? t('infoBox.smooth') : t('infoBox.corner')}
          </button>
        </div>
        <div className={styles.row}>
          <span className={styles.rowLabel}>{t('infoBox.curve')}</span>
          <span className={styles.badge}>{t('infoBox.onCurve')}</span>
        </div>
        <SmartCornerField
          target={target}
          outline={outline}
          points={[ref]}
          onCommitted={onCommitted}
        />
      </>
    )

  const node = nodeOfHandle(outline.contours, ref)
  const anchor = node ? pointAt(outline.contours, node) : null
  const offset = anchor
    ? { x: point.x - anchor.x, y: point.y - anchor.y }
    : { x: 0, y: 0 }
  const polar = anchor ? handlePolar(anchor, point) : { angle: 0, length: 0 }
  const move = (dx: number, dy: number) =>
    commitField(
      target,
      movePointsCommand(target, [ref], dx, dy, 'PointsOnly'),
      t('editor.undo.moveHandle'),
      onCommitted,
    )

  return (
    <>
      <NumberField
        label={t('infoBox.deltaX')}
        value={offset.x}
        invalidHint={t('infoBox.invalid')}
        onCommit={(next) => move(next - offset.x, 0)}
      />
      <NumberField
        label={t('infoBox.deltaY')}
        value={offset.y}
        invalidHint={t('infoBox.invalid')}
        onCommit={(next) => move(0, next - offset.y)}
      />
      <NumberField
        label={t('infoBox.length')}
        value={polar.length}
        invalidHint={t('infoBox.invalid')}
        onCommit={(next) => {
          const radians = (polar.angle * Math.PI) / 180
          move(
            Math.cos(radians) * next - offset.x,
            Math.sin(radians) * next - offset.y,
          )
        }}
      />
      <NumberField
        label={t('infoBox.angle')}
        value={polar.angle}
        invalidHint={t('infoBox.invalid')}
        onCommit={(next) => {
          const radians = (next * Math.PI) / 180
          move(
            Math.cos(radians) * polar.length - offset.x,
            Math.sin(radians) * polar.length - offset.y,
          )
        }}
      />
    </>
  )
}
