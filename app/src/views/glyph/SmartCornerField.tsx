import { useTranslation } from 'react-i18next'
import type { PointRef } from '../../api'
import { Button } from '../../components/Button'
import { NumberField } from '../../components/NumberField'
import { pointAt } from '../../editor/outlineGeometry'
import { commitField } from './commitField'
import type { InfoRowsProps } from './infoBoxProps'
import styles from './infoBox.module.css'

/** Signed live radius shared by the selected source nodes. */
export function SmartCornerField({
  target,
  outline,
  points,
  onCommitted,
}: Pick<InfoRowsProps, 'target' | 'outline' | 'onCommitted'> & {
  points: PointRef[]
}) {
  const { t } = useTranslation()
  const nodes = points.filter((ref) => {
    const p = pointAt(outline.contours, ref)
    return p && p.kind !== 'OffCurve'
  })
  if (!nodes.length) return null
  const radii = nodes.map(
    (ref) => pointAt(outline.contours, ref)?.corner_radius ?? 0,
  )
  const radius = radii[0] ?? 0
  const mixed = radii.some((r) => r !== radius)
  const apply = (radius: number) =>
    commitField(
      target,
      {
        SetSmartCorner: {
          glyph: target.glyph,
          layer: target.layer,
          points: nodes,
          radius,
        },
      },
      t('smartCorners.undo'),
      onCommitted,
    )
  return (
    <>
      <NumberField
        key={nodes.map((p) => `${p.contour}:${p.point}`).join(',')}
        label={t('smartCorners.radius')}
        value={radius}
        revision={outline.contours}
        step={1}
        invalidHint={t('smartCorners.help')}
        onCommit={apply}
      />
      <div className={styles.row}>
        <span className={styles.rowLabel}>
          {mixed
            ? t('smartCorners.mixed')
            : radius < 0
              ? t('smartCorners.trap')
              : radius > 0
                ? t('smartCorners.round')
                : t('smartCorners.none')}
        </span>
        <Button disabled={!radii.some((r) => r !== 0)} onClick={() => apply(0)}>
          {t('smartCorners.remove')}
        </Button>
      </div>
      {mixed && (
        <Button onClick={() => apply(radius)}>{t('smartCorners.unify')}</Button>
      )}
      <p className={styles.cornerHelp}>{t('smartCorners.help')}</p>
    </>
  )
}
