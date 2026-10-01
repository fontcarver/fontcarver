import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type BracketCondition,
  type DocId,
  type Layer,
  type LayerKind,
  type SmartAxis,
  type VariationState,
} from '../../../api'
import { Button } from '../../../components/Button'
import { Dialog } from '../../../components/Dialog'
import { NumberField } from '../../../components/NumberField'
import { activeEditor } from '../../../editor/activeEditor'
import { errorNotice } from '../../../i18n/errors'
import styles from '../../fontinfo/designSpace.module.css'
import { userToDesign } from '../../fontinfo/designCoordinates'
import { BracketConditionFields } from './BracketConditionFields'
import {
  canonicalConditions,
  conditionSetError,
  conditionsOf,
} from './bracketConditions'

/** Clone a source or edit its intermediate/conditional role without replacing its outline. */
export function SourceLayerEditor({
  doc,
  glyph,
  source,
  master,
  duplicate,
  smartAxes,
  onClose,
  onApplied,
}: {
  doc: DocId
  glyph: string
  source: Layer
  master: string
  smartAxes: SmartAxis[]
  duplicate: boolean
  onClose: () => void
  onApplied: () => void
}) {
  const { t } = useTranslation()
  const [state, setState] = useState<VariationState | null>(null)
  const [kind, setKind] = useState(
    'Smart' in source.kind ||
      (duplicate &&
        !('Brace' in source.kind) &&
        !('Bracket' in source.kind) &&
        smartAxes.length > 0)
      ? 'Smart'
      : 'Bracket' in source.kind
        ? 'Bracket'
        : 'Brace',
  )
  const [smartLocation, setSmartLocation] = useState<Record<string, number>>(
    'Smart' in source.kind ? source.kind.Smart.location : {},
  )
  const [name, setName] = useState(duplicate ? '' : source.name)
  const [location, setLocation] = useState<Record<string, number>>(
    'Brace' in source.kind ? source.kind.Brace.location : {},
  )
  const [axis, setAxis] = useState(
    'Bracket' in source.kind
      ? String.fromCharCode(...source.kind.Bracket.axis)
      : '',
  )
  const [min, setMin] = useState(
    'Bracket' in source.kind ? source.kind.Bracket.min : 0,
  )
  const [max, setMax] = useState<number | null>(
    'Bracket' in source.kind ? source.kind.Bracket.max : null,
  )
  // A condition is a set of per-axis ranges; the first is edited above, the
  // rest live here so this dialog offers the same compound condition the
  // variation panel does.
  const [extra, setExtra] = useState<BracketCondition[]>(
    'Bracket' in source.kind ? (source.kind.Bracket.extra ?? []) : [],
  )
  const [busy, setBusy] = useState(false),
    [error, setError] = useState('')
  const selectedAxis = state?.axes.find(
    (value) => String.fromCharCode(...value.tag) === axis,
  )
  const conditionError =
    kind === 'Bracket' &&
    conditionSetError(
      state?.axes ?? [],
      selectedAxis
        ? conditionsOf({
            Bracket: { axis: selectedAxis.tag, min, max, extra },
          })
        : [],
    )
  useEffect(() => {
    let current = true
    void Promise.resolve(activeEditor()?.settle())
      .then(() => call('variation.state', { doc }))
      .then((value) => {
        if (!current) return
        setState(value)
        const tags = value.axes.map((a) => String.fromCharCode(...a.tag))
        setAxis((current) => current || tags[0] || '')
        const origin = {
          ...Object.fromEntries(
            value.axes.map((axis, index) => [
              tags[index]!,
              userToDesign(axis, axis.default),
            ]),
          ),
          ...value.masters.find((m) => m.id === master)?.location,
        }
        setLocation((current) => ({ ...origin, ...current }))
        if (duplicate && value.axes[0])
          setMin(
            origin[tags[0]!] ??
              userToDesign(value.axes[0], value.axes[0].default),
          )
      })
      .catch((error: unknown) => {
        if (current) {
          const notice = errorNotice(error, t)
          setError(notice.details ?? notice.message)
        }
      })
    return () => {
      current = false
    }
  }, [doc, master, duplicate, t])
  const apply = () => {
    if (!state || busy || conditionError) return
    const role: LayerKind =
      kind === 'Smart'
        ? { Smart: { master, location: smartLocation } }
        : kind === 'Brace'
          ? { Brace: { master, location } }
          : {
              Bracket: {
                master,
                axis: [...axis].map((c) => c.charCodeAt(0)) as [
                  number,
                  number,
                  number,
                  number,
                ],
                min,
                max,
                extra: canonicalConditions(extra),
              },
            }
    setBusy(true)
    void call('variation.apply', {
      doc,
      expected_generation: state.generation,
      label: t(duplicate ? 'variation.addSource' : 'variation.editSource'),
      command: {
        Layer: { glyph, source: source.id, duplicate, kind: role, name },
      },
    })
      .then(() => {
        onApplied()
        onClose()
      })
      .catch((error: unknown) => {
        const notice = errorNotice(error, t)
        setError(notice.details ?? notice.message)
      })
      .finally(() => setBusy(false))
  }
  return (
    <Dialog
      title={t(duplicate ? 'variation.addSource' : 'variation.editSource')}
      size="md"
      onCancel={() => {
        if (!busy) onClose()
      }}
      footer={
        <>
          <Button onClick={onClose} disabled={busy}>
            {t('common.cancel')}
          </Button>
          <Button
            variant="primary"
            disabled={
              busy ||
              !state ||
              Boolean(conditionError) ||
              (kind === 'Smart' ? !smartAxes.length : !state.axes.length)
            }
            onClick={apply}
          >
            {t('variation.apply')}
          </Button>
        </>
      }
    >
      <fieldset disabled={busy} className={styles.form}>
        {error && <p role="alert">{error}</p>}
        <p>
          {t(
            kind === 'Smart'
              ? 'variation.smartSourceHint'
              : 'variation.sourceLayerHint',
          )}
        </p>
        <label>
          {t('variation.layerName')}
          <input value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <label>
          {t('variation.sourceRole')}
          <select value={kind} onChange={(e) => setKind(e.target.value)}>
            {smartAxes.length > 0 && (
              <option value="Smart">
                {t('glyphInspector.layerKind.smart')}
              </option>
            )}
            <option value="Brace">{t('glyphInspector.layerKind.brace')}</option>
            <option value="Bracket">
              {t('glyphInspector.layerKind.bracket')}
            </option>
          </select>
        </label>
        {kind === 'Brace' && Boolean(state?.virtual_masters?.length) && (
          <label>
            {t('virtualMaster.sourcePreset')}
            <select
              defaultValue=""
              onChange={(event) => {
                const preset =
                  state?.virtual_masters?.[Number(event.target.value)]
                if (!preset || !state) return
                setLocation(
                  Object.fromEntries(
                    state.axes.map((axis) => {
                      const tag = String.fromCharCode(...axis.tag)
                      return [
                        tag,
                        preset.location[tag] ??
                          userToDesign(axis, axis.default),
                      ]
                    }),
                  ),
                )
                if (!name) setName(preset.name)
              }}
            >
              <option value="" disabled>
                {t('virtualMaster.choosePreset')}
              </option>
              {state?.virtual_masters?.map((preset, index) => (
                <option key={index} value={index}>
                  {preset.name}
                </option>
              ))}
            </select>
          </label>
        )}
        {kind === 'Smart' ? (
          smartAxes.map((axis) => (
            <NumberField
              key={axis.name}
              label={axis.name}
              value={smartLocation[axis.name] ?? axis.default}
              onCommit={(value) =>
                setSmartLocation((current) => ({
                  ...current,
                  [axis.name]: value,
                }))
              }
            />
          ))
        ) : kind === 'Brace' ? (
          state?.axes.map((a) => {
            const tag = String.fromCharCode(...a.tag)
            return (
              <NumberField
                key={tag}
                label={tag}
                value={location[tag] ?? userToDesign(a, a.default)}
                onCommit={(number) =>
                  setLocation((current) => ({ ...current, [tag]: number }))
                }
              />
            )
          })
        ) : (
          <BracketConditionFields
            axes={state?.axes ?? []}
            axis={axis}
            min={min}
            max={max}
            location={location}
            extra={extra}
            onAxis={setAxis}
            onMin={setMin}
            onMax={setMax}
            onExtra={setExtra}
          />
        )}
      </fieldset>
    </Dialog>
  )
}
