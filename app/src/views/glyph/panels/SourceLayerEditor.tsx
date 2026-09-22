import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
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
  const [busy, setBusy] = useState(false),
    [error, setError] = useState('')
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
    if (!state || busy) return
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
          <>
            <label>
              {t('variation.conditionAxis')}
              <select value={axis} onChange={(e) => setAxis(e.target.value)}>
                {state?.axes.map((a) => {
                  const tag = String.fromCharCode(...a.tag)
                  return <option key={tag}>{tag}</option>
                })}
              </select>
            </label>
            <label>
              <input
                type="checkbox"
                checked={min !== null}
                onChange={(e) =>
                  setMin(e.target.checked ? (location[axis] ?? 0) : null)
                }
              />
              {t('variation.lowerBound')}
            </label>
            {min !== null && (
              <NumberField
                label={t('variation.lowerBound')}
                value={min}
                onCommit={setMin}
              />
            )}
            <label>
              <input
                type="checkbox"
                checked={max !== null}
                onChange={(e) =>
                  setMax(e.target.checked ? (location[axis] ?? 0) : null)
                }
              />
              {t('variation.upperBound')}
            </label>
            {max !== null && (
              <NumberField
                label={t('variation.upperBound')}
                value={max}
                onCommit={setMax}
              />
            )}
          </>
        )}
      </fieldset>
    </Dialog>
  )
}
