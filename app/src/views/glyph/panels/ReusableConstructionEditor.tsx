import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  call,
  type ConstructionCreate,
  type ConstructionCreateKind,
  type GlyphConstruction,
  type Layer,
  type LayerEffectsResult,
  type PointRef,
} from '../../../api'
import { Button } from '../../../components/Button'
import { Dialog } from '../../../components/Dialog'
import { NumberField } from '../../../components/NumberField'
import { Select } from '../../../components/Select'
import { activeEditor } from '../../../editor/activeEditor'
import type { LayerTarget } from '../../../editor/pointCommands'
import { errorNotice } from '../../../i18n/errors'
import { StylePreview } from '../../style/StylePreview'
import { GlyphPicker } from './GlyphPicker'
import styles from './layerEffects.module.css'

interface Props {
  target: LayerTarget
  layer: Layer
  points: readonly PointRef[]
  onClose: () => void
  onApplied: () => void
}

type Mode = ConstructionCreateKind['kind']

/** Preview and generation-checked commit of reusable source glyph attachments. */
export function ReusableConstructionEditor({
  target,
  layer,
  points,
  onClose,
  onApplied,
}: Props) {
  const { t } = useTranslation()
  const [entries, setEntries] = useState<GlyphConstruction[]>(
    layer.constructions ?? [],
  )
  const [source, setSource] = useState('')
  const [picking, setPicking] = useState(false)
  const [mode, setMode] = useState<Mode>('corner')
  const [scale, setScale] = useState(1)
  const [spacing, setSpacing] = useState(100)
  const [sourceLength, setSourceLength] = useState(100)
  const [keepCarrier, setKeepCarrier] = useState(true)
  const [preview, setPreview] = useState<{
    result: LayerEffectsResult
    entries: GlyphConstruction[]
    add: ConstructionCreate | null
  } | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')

  const changed = (fn: () => void) => {
    setPreview(null)
    setError('')
    fn()
  }
  const add = (): ConstructionCreate | null => {
    if (!source) return null
    const at = points[0]
    if (
      !at ||
      (mode === 'segment' && points.length !== 2) ||
      (mode !== 'segment' && points.length !== 1)
    )
      throw new Error(t('construction.selectionRequired'))
    const kind: ConstructionCreateKind =
      mode === 'segment'
        ? {
            kind: mode,
            start: at,
            end: points[1]!,
            source_length: sourceLength,
          }
        : mode === 'brush'
          ? { kind: mode, at, spacing, scale }
          : { kind: mode, at, scale }
    return { source, kind, keep_carrier: keepCarrier }
  }
  const report = (cause: unknown) => setError(errorNotice(cause, t).message)
  const review = async () => {
    setBusy(true)
    setPreview(null)
    setError('')
    try {
      await activeEditor()?.settle()
      const attachment = add()
      const result = await call('constructions.apply', {
        ...target,
        constructions: entries,
        add: attachment,
        dry_run: true,
        bake: false,
        expected_generation: null,
      })
      setPreview({ result, entries, add: attachment })
    } catch (cause) {
      report(cause)
    } finally {
      setBusy(false)
    }
  }
  const apply = async (bake: boolean) => {
    if (!preview) return
    setBusy(true)
    setError('')
    try {
      await call('constructions.apply', {
        ...target,
        constructions: preview.entries,
        add: preview.add,
        dry_run: false,
        bake,
        expected_generation: preview.result.generation,
      })
      onApplied()
      onClose()
    } catch (cause) {
      setPreview(null)
      report(cause)
    } finally {
      setBusy(false)
    }
  }
  const update = (index: number, next: GlyphConstruction) =>
    changed(() =>
      setEntries(entries.map((entry, i) => (i === index ? next : entry))),
    )

  return (
    <Dialog
      title={t('construction.editorTitle')}
      size="lg"
      scrollBody
      onCancel={() => {
        if (!busy) onClose()
      }}
      footer={
        <>
          <Button disabled={busy} onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button
            disabled={busy}
            onClick={() => {
              void review()
            }}
          >
            {t('construction.preview')}
          </Button>
          <Button
            disabled={busy || !preview}
            onClick={() => {
              void apply(true)
            }}
          >
            {t('construction.bake')}
          </Button>
          <Button
            variant="primary"
            disabled={busy || !preview}
            onClick={() => {
              void apply(false)
            }}
          >
            {t('construction.keepLive')}
          </Button>
        </>
      }
    >
      <div className={styles.editor}>
        <p>
          <strong>
            {target.glyph} · {layer.name}
          </strong>
        </p>
        <p>{t('construction.geometryHelp')}</p>
        <fieldset disabled={busy} className={styles.form}>
          <legend>{t('construction.existing')}</legend>
          {entries.length === 0 && <p>{t('construction.empty')}</p>}
          {entries.map((entry, index) => {
            const kind = entry.kind
            const variant = Object.keys(kind)[0] as keyof typeof kind
            const data =
              'Corner' in kind
                ? kind.Corner
                : 'Cap' in kind
                  ? kind.Cap
                  : 'Brush' in kind
                    ? kind.Brush
                    : kind.Segment
            return (
              <section
                className={styles.effect}
                key={entry.id}
                aria-label={entry.source}
              >
                <div className={styles.row}>
                  <strong>
                    {entry.source} ·{' '}
                    {t(`construction.kind.${String(variant).toLowerCase()}`)}
                  </strong>
                  <Button
                    onClick={() =>
                      changed(() =>
                        setEntries(entries.filter((_, i) => i !== index)),
                      )
                    }
                  >
                    {t('construction.remove')}
                  </Button>
                </div>
                {'scale' in data && (
                  <NumberField
                    label={t('construction.scale')}
                    value={data.scale}
                    onCommit={(value) => {
                      if ('Corner' in kind)
                        update(index, {
                          ...entry,
                          kind: { Corner: { ...kind.Corner, scale: value } },
                        })
                      else if ('Cap' in kind)
                        update(index, {
                          ...entry,
                          kind: { Cap: { ...kind.Cap, scale: value } },
                        })
                      else if ('Brush' in kind)
                        update(index, {
                          ...entry,
                          kind: { Brush: { ...kind.Brush, scale: value } },
                        })
                    }}
                  />
                )}
                {'Brush' in kind && (
                  <NumberField
                    label={t('construction.spacing')}
                    value={kind.Brush.spacing}
                    onCommit={(value) =>
                      update(index, {
                        ...entry,
                        kind: { Brush: { ...kind.Brush, spacing: value } },
                      })
                    }
                  />
                )}
                {'Segment' in kind && (
                  <NumberField
                    label={t('construction.baseline')}
                    value={kind.Segment.source_length}
                    onCommit={(value) =>
                      update(index, {
                        ...entry,
                        kind: {
                          Segment: { ...kind.Segment, source_length: value },
                        },
                      })
                    }
                  />
                )}
                <label>
                  <input
                    type="checkbox"
                    checked={entry.keep_carrier}
                    onChange={(event) =>
                      update(index, {
                        ...entry,
                        keep_carrier: event.target.checked,
                      })
                    }
                  />
                  {t('construction.keepCarrier')}
                </label>
              </section>
            )
          })}
        </fieldset>
        <fieldset disabled={busy} className={styles.form}>
          <legend>{t('construction.newAttachment')}</legend>
          <p>{t('construction.selectionHelp', { count: points.length })}</p>
          <Select
            label={t('construction.mode')}
            value={mode}
            options={(['corner', 'cap', 'segment', 'brush'] as const).map(
              (kind) => ({
                value: kind,
                label: t(`construction.kind.${kind}`),
              }),
            )}
            onChange={(value) => changed(() => setMode(value))}
          />
          <Button onClick={() => setPicking(true)}>
            {source || t('construction.chooseSource')}
          </Button>
          {source && (
            <Button onClick={() => changed(() => setSource(''))}>
              {t('construction.clearSource')}
            </Button>
          )}
          {picking && (
            <GlyphPicker
              doc={target.doc}
              label={t('construction.chooseSource')}
              onClose={() => setPicking(false)}
              onPick={(name) =>
                changed(() => {
                  setSource(name)
                  setPicking(false)
                })
              }
            />
          )}
          {mode === 'segment' ? (
            <NumberField
              label={t('construction.baseline')}
              value={sourceLength}
              onCommit={(value) => changed(() => setSourceLength(value))}
            />
          ) : (
            <NumberField
              label={t('construction.scale')}
              value={scale}
              onCommit={(value) => changed(() => setScale(value))}
            />
          )}
          {mode === 'brush' && (
            <NumberField
              label={t('construction.spacing')}
              value={spacing}
              onCommit={(value) => changed(() => setSpacing(value))}
            />
          )}
          <label>
            <input
              type="checkbox"
              checked={keepCarrier}
              onChange={(event) =>
                changed(() => setKeepCarrier(event.target.checked))
              }
            />
            {t('construction.keepCarrier')}
          </label>
        </fieldset>
        {error && (
          <p role="alert" className={styles.error}>
            {error}
          </p>
        )}
        {busy && <p role="status">{t('construction.working')}</p>}
        {preview && (
          <>
            <p role="status">{t('construction.reviewed')}</p>
            <StylePreview
              example={preview.result.example}
              upem={1000}
              showSource
            />
          </>
        )}
      </div>
    </Dialog>
  )
}
