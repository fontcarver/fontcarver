import { useTranslation } from 'react-i18next'
import type { Axis, BracketCondition } from '../../../api'
import { Button } from '../../../components/Button'
import { NumberField } from '../../../components/NumberField'
import { userToDesign } from '../../fontinfo/designCoordinates'
import {
  axisBounds,
  axisTag,
  bracketConditionError,
  conditionSetError,
  formatConditionSet,
} from './bracketConditions'

/**
 * One inclusive axis range.
 *
 * The first range of a set keeps the plain field labels the single-axis editor
 * has always used; an added range qualifies its labels with the axis tag so the
 * form still has one accessible name per control.
 */
function ConditionRow({
  axes,
  condition,
  used,
  origin,
  qualified,
  onChange,
  onRemove,
}: {
  axes: Axis[]
  condition: BracketCondition
  used: readonly string[]
  origin: number
  qualified: boolean
  onChange: (next: BracketCondition) => void
  onRemove?: () => void
}) {
  const { t } = useTranslation()
  const tag = axisTag(condition.axis)
  const selected = axes.find((value) => axisTag(value.tag) === tag)
  const [low, high] = selected ? axisBounds(selected) : [0, 0]
  const min = condition.min ?? null
  const max = condition.max ?? null
  const label = (bound: 'lowerBound' | 'upperBound') =>
    qualified
      ? t(`variation.${bound}For`, { axis: tag })
      : t(`variation.${bound}`)
  return (
    <>
      <label>
        {t('variation.conditionAxis')}
        <select
          value={tag}
          onChange={(event) => {
            const next = axes.find(
              (value) => axisTag(value.tag) === event.target.value,
            )
            if (!next) return
            const [nextLow, nextHigh] = axisBounds(next)
            onChange({
              axis: next.tag,
              min:
                min === null
                  ? undefined
                  : Math.min(
                      Math.max(userToDesign(next, next.default), nextLow),
                      nextHigh,
                    ),
              max: max === null ? undefined : nextHigh,
            })
          }}
        >
          {axes
            .filter(
              (value) =>
                axisTag(value.tag) === tag ||
                !used.includes(axisTag(value.tag)),
            )
            .map((value) => (
              <option key={axisTag(value.tag)} value={axisTag(value.tag)}>
                {value.name} ({axisTag(value.tag)})
              </option>
            ))}
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
          onChange={(event) =>
            onChange({
              ...condition,
              min: event.target.checked ? origin : undefined,
            })
          }
        />
        {label('lowerBound')}
      </label>
      {min !== null && (
        <NumberField
          label={label('lowerBound')}
          value={min}
          onCommit={(value) => onChange({ ...condition, min: value })}
        />
      )}
      <label>
        <input
          type="checkbox"
          checked={max !== null}
          onChange={(event) =>
            onChange({
              ...condition,
              max: event.target.checked ? high : undefined,
            })
          }
        />
        {label('upperBound')}
      </label>
      {max !== null && (
        <NumberField
          label={label('upperBound')}
          value={max}
          onCommit={(value) => onChange({ ...condition, max: value })}
        />
      )}
      {onRemove && (
        <Button onClick={onRemove}>
          {t('variation.removeCondition', { axis: tag })}
        </Button>
      )}
    </>
  )
}

/**
 * The alternate-layer condition editor.
 *
 * The primary axis range is always present. When `extra` and `onExtra` are
 * given, further axis ranges can be added and removed and the set reads as a
 * conjunction — the `[80‹wght, 25‹wdth‹50]` form Glyphs writes.
 */
export function BracketConditionFields({
  axes,
  axis,
  min,
  max,
  location,
  extra,
  onAxis,
  onMin,
  onMax,
  onExtra,
}: {
  axes: Axis[]
  axis: string
  min: number | null
  max: number | null
  location: Record<string, number>
  extra?: BracketCondition[]
  onAxis: (value: string) => void
  onMin: (value: number | null) => void
  onMax: (value: number | null) => void
  onExtra?: (value: BracketCondition[]) => void
}) {
  const { t } = useTranslation()
  const compound = extra !== undefined && onExtra !== undefined
  const selected = axes.find((value) => axisTag(value.tag) === axis)
  const [low, high] = selected ? axisBounds(selected) : [0, 0]
  const originOf = (tag: string, value: Axis | undefined) =>
    value ? (location[tag] ?? userToDesign(value, value.default)) : 0
  const rows: BracketCondition[] = [
    ...(selected
      ? [{ axis: selected.tag, min: min ?? undefined, max: max ?? undefined }]
      : []),
    ...(extra ?? []),
  ]
  const used = rows.map((row) => axisTag(row.axis))
  const unused = axes.filter((value) => !used.includes(axisTag(value.tag)))
  const error = compound
    ? conditionSetError(axes, rows)
    : bracketConditionError(selected, min, max)
  return (
    <>
      {selected ? (
        <ConditionRow
          axes={axes}
          condition={{
            axis: selected.tag,
            min: min ?? undefined,
            max: max ?? undefined,
          }}
          used={used}
          origin={originOf(axis, selected)}
          qualified={false}
          onChange={(next) => {
            const tag = axisTag(next.axis)
            if (tag !== axis) onAxis(tag)
            onMin(next.min ?? null)
            onMax(next.max ?? null)
          }}
        />
      ) : (
        <label>
          {t('variation.conditionAxis')}
          <select value={axis} onChange={(event) => onAxis(event.target.value)}>
            {axes.map((value) => (
              <option key={axisTag(value.tag)} value={axisTag(value.tag)}>
                {value.name} ({axisTag(value.tag)})
              </option>
            ))}
          </select>
        </label>
      )}
      {compound &&
        extra.map((condition, index) => (
          <ConditionRow
            key={`${axisTag(condition.axis)}:${index}`}
            axes={axes}
            condition={condition}
            used={used}
            qualified
            origin={originOf(
              axisTag(condition.axis),
              axes.find(
                (value) => axisTag(value.tag) === axisTag(condition.axis),
              ),
            )}
            onChange={(next) =>
              onExtra(
                extra.map((existing, position) =>
                  position === index ? next : existing,
                ),
              )
            }
            onRemove={() =>
              onExtra(extra.filter((_, position) => position !== index))
            }
          />
        ))}
      {compound && unused.length > 0 && (
        <Button
          onClick={() => {
            const next = unused[0]
            if (!next) return
            onExtra([
              ...extra,
              { axis: next.tag, min: userToDesign(next, next.default) },
            ])
          }}
        >
          {t('variation.addCondition')}
        </Button>
      )}
      {error ? (
        <p role="alert">{t(error, { min: low, max: high })}</p>
      ) : (
        <p role="status">
          {t('variation.conditionSummary', {
            condition: formatConditionSet(rows),
          })}
        </p>
      )}
    </>
  )
}
