import { useTranslation } from 'react-i18next'
import type { Axis } from '../../../api'
import { NumberField } from '../../../components/NumberField'
import { userToDesign } from '../../fontinfo/designCoordinates'
import { bracketConditionError } from './bracketConditions'

/** A bracket condition uses mapped design units, with explicit inclusive ends. */
export function BracketConditionFields({
  axes,
  axis,
  min,
  max,
  location,
  onAxis,
  onMin,
  onMax,
}: {
  axes: Axis[]
  axis: string
  min: number | null
  max: number | null
  location: Record<string, number>
  onAxis: (value: string) => void
  onMin: (value: number | null) => void
  onMax: (value: number | null) => void
}) {
  const { t } = useTranslation()
  const selected = axes.find(
    (value) => String.fromCharCode(...value.tag) === axis,
  )
  const low = selected ? userToDesign(selected, selected.min) : 0
  const high = selected ? userToDesign(selected, selected.max) : 0
  const origin = selected
    ? (location[axis] ?? userToDesign(selected, selected.default))
    : 0
  const error = bracketConditionError(selected, min, max)
  const condition =
    min !== null && max !== null
      ? `${min} ≤ ${axis} ≤ ${max}`
      : min !== null
        ? `${axis} ≥ ${min}`
        : `${axis} ≤ ${max}`
  return (
    <>
      <label>
        {t('variation.conditionAxis')}
        <select
          value={axis}
          onChange={(event) => {
            const tag = event.target.value
            const next = axes.find(
              (value) => String.fromCharCode(...value.tag) === tag,
            )
            onAxis(tag)
            if (next) {
              if (min !== null) onMin(userToDesign(next, next.default))
              if (max !== null) onMax(userToDesign(next, next.max))
            }
          }}
        >
          {axes.map((value) => {
            const tag = String.fromCharCode(...value.tag)
            return (
              <option key={tag} value={tag}>
                {value.name} ({tag})
              </option>
            )
          })}
        </select>
      </label>
      <p>
        {t('variation.conditionRange', {
          min: low,
          max: high,
          userMin: selected?.min,
          userMax: selected?.max,
        })}
      </p>
      <label>
        <input
          type="checkbox"
          checked={min !== null}
          onChange={(event) => onMin(event.target.checked ? origin : null)}
        />
        {t('variation.lowerBound')}
      </label>
      {min !== null && (
        <NumberField
          label={t('variation.lowerBound')}
          value={min}
          onCommit={onMin}
        />
      )}
      <label>
        <input
          type="checkbox"
          checked={max !== null}
          onChange={(event) => onMax(event.target.checked ? high : null)}
        />
        {t('variation.upperBound')}
      </label>
      {max !== null && (
        <NumberField
          label={t('variation.upperBound')}
          value={max}
          onCommit={onMax}
        />
      )}
      {error ? (
        <p role="alert">{t(error, { min: low, max: high })}</p>
      ) : (
        <p role="status">{t('variation.conditionSummary', { condition })}</p>
      )}
    </>
  )
}
