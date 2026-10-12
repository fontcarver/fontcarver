import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import type { LayerEffectKind } from '../../../api'
import { Button } from '../../../components/Button'
import { Select } from '../../../components/Select'
import styles from './layerEffects.module.css'

type VariableStroke = Extract<LayerEffectKind, { kind: 'variable_stroke' }>
type PenPoint = VariableStroke['points'][number]

/** Editable pen samples along each source contour's normalized arc length. */
export function PenPointFields({
  effect,
  onChange,
}: {
  effect: VariableStroke
  onChange: (effect: VariableStroke) => void
}) {
  const { t } = useTranslation()
  const update = (index: number, key: keyof PenPoint, value: number) =>
    onChange({
      ...effect,
      points: effect.points.map((point, i) =>
        i === index ? { ...point, [key]: value } : point,
      ),
    })
  const add = () => {
    let interval = 0
    for (let i = 1; i < effect.points.length - 1; i++) {
      if (
        effect.points[i + 1]!.position - effect.points[i]!.position >
        effect.points[interval + 1]!.position -
          effect.points[interval]!.position
      )
        interval = i
    }
    const start = effect.points[interval]!
    const end = effect.points[interval + 1]!
    const angleDelta = ((((end.angle - start.angle) % 360) + 540) % 360) - 180
    const points = [...effect.points]
    points.splice(interval + 1, 0, {
      position: (start.position + end.position) / 2,
      width: (start.width + end.width) / 2,
      height: (start.height + end.height) / 2,
      angle: ((((start.angle + angleDelta / 2 + 180) % 360) + 360) % 360) - 180,
    })
    onChange({ ...effect, points })
  }
  return (
    <div className={styles.penPoints}>
      <div className={styles.numbers}>
        <label>
          {t('layerEffects.progression')}
          <Select
            label={t('layerEffects.progression')}
            value={effect.progression}
            options={(['linear', 'smooth'] as const).map((value) => ({
              value,
              label: t(`layerEffects.${value}`),
            }))}
            onChange={(progression) => onChange({ ...effect, progression })}
          />
        </label>
      </div>
      <p>{t('layerEffects.pointsHelp')}</p>
      {effect.points.map((point, index) => (
        <fieldset className={styles.penPoint} key={index}>
          <legend>{t('layerEffects.point', { index: index + 1 })}</legend>
          <div className={styles.numbers}>
            {(['position', 'width', 'height', 'angle'] as const).map((key) => (
              <label key={key}>
                {t(`layerEffects.${key}`)}
                <input
                  type="number"
                  value={Number.isFinite(point[key]) ? point[key] : ''}
                  step={key === 'position' ? 0.01 : 0.1}
                  min={key === 'position' ? 0 : key === 'angle' ? -360 : 0.1}
                  max={key === 'position' ? 1 : key === 'angle' ? 360 : 8192}
                  disabled={
                    key === 'position' &&
                    (index === 0 || index === effect.points.length - 1)
                  }
                  onChange={(event) =>
                    update(index, key, event.target.valueAsNumber)
                  }
                />
              </label>
            ))}
          </div>
          <Button
            aria-label={t('layerEffects.removePoint', { index: index + 1 })}
            disabled={index === 0 || index === effect.points.length - 1}
            onClick={() =>
              onChange({
                ...effect,
                points: effect.points.filter((_, i) => i !== index),
              })
            }
          >
            <X size={14} aria-hidden="true" />
            {t('layerEffects.removePoint', { index: index + 1 })}
          </Button>
        </fieldset>
      ))}
      <div className={styles.actions}>
        <Button
          disabled={
            effect.points.length >= 64 ||
            effect.points.length < 2 ||
            effect.points.some(
              (point, index) =>
                Object.values(point).some((value) => !Number.isFinite(value)) ||
                (index > 0 &&
                  point.position <= effect.points[index - 1]!.position),
            )
          }
          onClick={add}
        >
          {t('layerEffects.addPoint')}
        </Button>
      </div>
      <p>{t('layerEffects.progressionHelp')}</p>
    </div>
  )
}
