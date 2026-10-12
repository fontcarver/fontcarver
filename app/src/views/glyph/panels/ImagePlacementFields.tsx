import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { Image } from '../../../api'
import {
  about,
  composeAffine,
  rotateAbout,
  scaleAbout,
  type Affine,
} from '../../../editor/affine'

import styles from './imagePanel.module.css'
import panel from './panels.module.css'

const FIELDS = [
  'scaleX',
  'shearY',
  'shearX',
  'scaleY',
  'translateX',
  'translateY',
] as const

/** Centre of the image in its own pixel frame, which every step turns about. */
function centre(size: { width: number; height: number }) {
  return { x: size.width / 2, y: size.height / 2 }
}

/**
 * Keyboard placement: the same move, scale, rotate and skew the canvas handles
 * perform, plus the six coefficients for an exact placement.
 *
 * Every button writes one `image.set`, so one press is one undo entry, and the
 * steps compose onto the stored transform rather than replacing it.
 */
export function ImagePlacementFields({
  image,
  size,
  disabled,
  onTransform,
}: {
  image: Image
  size: { width: number; height: number } | null
  disabled: boolean
  onTransform: (transform: Affine) => void
}) {
  const { t } = useTranslation()
  const [draft, setDraft] = useState<{
    source: string
    values: string[]
  } | null>(null)
  const [move, setMove] = useState(10)
  const [scale, setScale] = useState(105)
  const [angle, setAngle] = useState(5)
  const transformKey = JSON.stringify(image.transform)
  const placement =
    draft?.source === transformKey ? draft.values : image.transform.map(String)
  const typed = placement.map((value) =>
    value.trim() === '' ? NaN : Number(value),
  ) as Affine
  const pivot = centre(size ?? { width: 0, height: 0 })
  const step = (next: Affine) => onTransform(next)
  const local = (linear: Affine) =>
    step(composeAffine(image.transform as Affine, about(pivot, linear)))
  const actions: { key: string; label: string; run: () => void }[] = [
    {
      key: 'left',
      label: t('imageWorkflow.stepLeft'),
      run: () => {
        const next = [...image.transform] as Affine
        next[4] -= move
        step(next)
      },
    },
    {
      key: 'right',
      label: t('imageWorkflow.stepRight'),
      run: () => {
        const next = [...image.transform] as Affine
        next[4] += move
        step(next)
      },
    },
    {
      key: 'down',
      label: t('imageWorkflow.stepDown'),
      run: () => {
        const next = [...image.transform] as Affine
        next[5] -= move
        step(next)
      },
    },
    {
      key: 'up',
      label: t('imageWorkflow.stepUp'),
      run: () => {
        const next = [...image.transform] as Affine
        next[5] += move
        step(next)
      },
    },
    {
      key: 'scale',
      label: t('imageWorkflow.stepScale'),
      run: () =>
        step(
          composeAffine(
            image.transform as Affine,
            scaleAbout(pivot, scale / 100, scale / 100),
          ),
        ),
    },
    {
      key: 'rotate',
      label: t('imageWorkflow.stepRotate'),
      run: () =>
        step(
          composeAffine(
            rotateAbout(
              {
                x:
                  image.transform[0] * pivot.x +
                  image.transform[2] * pivot.y +
                  image.transform[4],
                y:
                  image.transform[1] * pivot.x +
                  image.transform[3] * pivot.y +
                  image.transform[5],
              },
              (angle * Math.PI) / 180,
            ),
            image.transform as Affine,
          ),
        ),
    },
    {
      key: 'skewX',
      label: t('imageWorkflow.stepSkewX'),
      run: () => local([1, 0, Math.tan((angle * Math.PI) / 180), 1, 0, 0]),
    },
    {
      key: 'skewY',
      label: t('imageWorkflow.stepSkewY'),
      run: () => local([1, Math.tan((angle * Math.PI) / 180), 0, 1, 0, 0]),
    },
  ]
  const stepsUsable = size !== null && !disabled
  return (
    <>
      <div className={styles.matrix}>
        <label className={styles.field}>
          {t('imageWorkflow.moveStep')}
          <input
            type="number"
            step="any"
            className={panel.textInput}
            value={Number.isFinite(move) ? move : ''}
            onChange={(event) => setMove(event.target.valueAsNumber)}
          />
        </label>
        <label className={styles.field}>
          {t('imageWorkflow.scaleStep')}
          <input
            type="number"
            step="any"
            className={panel.textInput}
            value={Number.isFinite(scale) ? scale : ''}
            onChange={(event) => setScale(event.target.valueAsNumber)}
          />
        </label>
        <label className={styles.field}>
          {t('imageWorkflow.angleStep')}
          <input
            type="number"
            step="any"
            className={panel.textInput}
            value={Number.isFinite(angle) ? angle : ''}
            onChange={(event) => setAngle(event.target.valueAsNumber)}
          />
        </label>
      </div>
      <div className={styles.steps}>
        {actions.map((action) => (
          <button
            key={action.key}
            type="button"
            className={panel.actionButton}
            disabled={
              !stepsUsable ||
              !Number.isFinite(move) ||
              !Number.isFinite(scale) ||
              !Number.isFinite(angle)
            }
            onClick={action.run}
          >
            {action.label}
          </button>
        ))}
      </div>
      {size === null && (
        <p className={styles.hint}>{t('imageWorkflow.stepsNeedSource')}</p>
      )}
      <div className={styles.matrix}>
        {FIELDS.map((field, index) => (
          <label key={field} className={styles.field}>
            {t(`imageWorkflow.${field}`)}
            <input
              type="number"
              step="any"
              className={panel.textInput}
              value={placement[index]}
              onChange={(event) =>
                setDraft({
                  source: transformKey,
                  values: placement.map((old, position) =>
                    position === index ? event.target.value : old,
                  ),
                })
              }
            />
          </label>
        ))}
      </div>
      <p className={styles.hint}>{t('imageWorkflow.affineHelp')}</p>
      <button
        type="button"
        className={panel.actionButton}
        disabled={disabled || !typed.every(Number.isFinite)}
        onClick={() => onTransform(typed)}
      >
        {t('imageWorkflow.applyPlacement')}
      </button>
    </>
  )
}
