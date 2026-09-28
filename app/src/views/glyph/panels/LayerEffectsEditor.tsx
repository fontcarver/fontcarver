import { useState } from 'react'
import { ArrowUp, ArrowDown, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type DocId,
  type Glyph,
  type Layer,
  type LayerEffect,
  type LayerEffectKind,
  type LayerEffectsResult,
} from '../../../api'
import { Button } from '../../../components/Button'
import { Dialog } from '../../../components/Dialog'
import { LayerEffectFields } from './LayerEffectFields'
import { activeEditor } from '../../../editor/activeEditor'
import { errorNotice } from '../../../i18n/errors'
import { StylePreview } from '../../style/StylePreview'
import styles from './layerEffects.module.css'

function initial(kind: LayerEffectKind['kind']): LayerEffect {
  return {
    enabled: true,
    effect:
      kind === 'stroke'
        ? {
            kind,
            nib: 'ellipse',
            width: 40,
            height: 40,
            angle: 0,
            cap: 'round',
            join: 'round',
            miter_limit: 4,
          }
        : kind === 'boolean'
          ? { kind, operation: 'union', path: '' }
          : { kind },
  }
}

/** Ordered source-preserving effects, applied only against a reviewed document revision. */
export function LayerEffectsEditor({
  doc,
  glyph,
  layer,
  upem,
  onClose,
  onApplied,
}: {
  doc: DocId
  glyph: Glyph
  layer: Layer
  upem: number
  onClose: () => void
  onApplied: () => void
}) {
  const { t } = useTranslation()
  const [effects, setEffects] = useState<LayerEffect[]>(layer.effects ?? [])
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [preview, setPreview] = useState<{
    effects: LayerEffect[]
    result: LayerEffectsResult
  } | null>(null)
  const change = (next: LayerEffect[]) => {
    setEffects(next)
    setPreview(null)
    setError('')
  }
  const update = (index: number, entry: LayerEffect) =>
    change(effects.map((old, i) => (i === index ? entry : old)))
  const move = (index: number, delta: number) => {
    const next = [...effects]
    const other = index + delta
    ;[next[index], next[other]] = [next[other]!, next[index]!]
    change(next)
  }
  const report = (error: unknown) => {
    const notice = errorNotice(error, t)
    setError([notice.message, notice.details].filter(Boolean).join(' · '))
  }
  const review = async () => {
    setBusy(true)
    setError('')
    setPreview(null)
    try {
      await activeEditor()?.settle()
      const result = await call('effects.apply', {
        doc,
        glyph: glyph.name,
        layer: layer.id,
        effects,
        dry_run: true,
        bake: false,
        expected_generation: null,
      })
      setPreview({ effects, result })
    } catch (error) {
      report(error)
    } finally {
      setBusy(false)
    }
  }
  const apply = async (bake: boolean) => {
    if (!preview) return
    setBusy(true)
    setError('')
    try {
      await call('effects.apply', {
        doc,
        glyph: glyph.name,
        layer: layer.id,
        effects: preview.effects,
        dry_run: false,
        bake,
        expected_generation: preview.result.generation,
      })
      onApplied()
      onClose()
    } catch (error) {
      setPreview(null)
      report(error)
    } finally {
      setBusy(false)
    }
  }
  return (
    <Dialog
      title={t('layerEffects.title')}
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
            {t('cjk.batchPreview')}
          </Button>
          <Button
            disabled={busy || !preview}
            onClick={() => {
              void apply(true)
            }}
          >
            {t('layerEffects.bake')}
          </Button>
          <Button
            variant="primary"
            disabled={busy || !preview}
            onClick={() => {
              void apply(false)
            }}
          >
            {t('variation.apply')}
          </Button>
        </>
      }
    >
      <div className={styles.editor}>
        <p>
          <strong>
            {glyph.name} · {layer.name || t('layerEffects.layer')}
          </strong>
        </p>
        <p>{t('layerEffects.description')}</p>
        <p>{t('layerEffects.bakeHelp')}</p>
        {layer.contours.some((c) =>
          c.points.some((p) => (p.corner_radius ?? 0) !== 0),
        ) && <p>{t('smartCorners.bakeNotice')}</p>}
        <fieldset disabled={busy} className={styles.form}>
          <legend className={styles.legend}>{t('layerEffects.stack')}</legend>
          <div className={styles.actions}>
            {(['stroke', 'remove_overlap', 'boolean'] as const).map((kind) => (
              <Button
                key={kind}
                disabled={effects.length >= 16}
                onClick={() => change([...effects, initial(kind)])}
              >
                {t(`layerEffects.add.${kind}`)}
              </Button>
            ))}
          </div>
          {effects.length === 0 && <p>{t('layerEffects.empty')}</p>}
          {effects.map((entry, index) => (
            <section
              className={styles.effect}
              key={index}
              aria-label={`${index + 1}. ${t(`layerEffects.kind.${entry.effect.kind}`)}`}
            >
              <div className={styles.row}>
                <label>
                  <input
                    type="checkbox"
                    checked={entry.enabled}
                    onChange={(e) =>
                      update(index, { ...entry, enabled: e.target.checked })
                    }
                  />
                  {index + 1}. {t(`layerEffects.kind.${entry.effect.kind}`)}
                </label>
                <div className={styles.actions}>
                  <Button
                    disabled={index === 0}
                    aria-label={t('layerEffects.up', { index: index + 1 })}
                    onClick={() => move(index, -1)}
                  >
                    <ArrowUp size={14} aria-hidden="true" />
                  </Button>
                  <Button
                    disabled={index === effects.length - 1}
                    aria-label={t('layerEffects.down', { index: index + 1 })}
                    onClick={() => move(index, 1)}
                  >
                    <ArrowDown size={14} aria-hidden="true" />
                  </Button>
                  <Button
                    aria-label={t('layerEffects.remove', { index: index + 1 })}
                    onClick={() =>
                      change(effects.filter((_, i) => i !== index))
                    }
                  >
                    <X size={14} aria-hidden="true" />
                  </Button>
                </div>
              </div>
              <LayerEffectFields
                effect={entry.effect}
                onChange={(effect) => update(index, { ...entry, effect })}
                doc={doc}
                glyph={glyph}
                onError={report}
                onBusy={setBusy}
              />
            </section>
          ))}
        </fieldset>
        {error && (
          <p role="alert" className={styles.error}>
            {error}
          </p>
        )}
        {busy && <p role="status">{t('layerEffects.working')}</p>}
        {preview && (
          <>
            <p role="status">{t('layerEffects.reviewed')}</p>
            {preview.result.approximated_layers.length > 0 && (
              <p role="note" className={styles.warning}>
                {t('layerEffects.fallback', {
                  count: preview.result.approximated_layers.length,
                })}
              </p>
            )}
            <StylePreview
              example={preview.result.example}
              upem={upem}
              showSource
            />
          </>
        )}
      </div>
    </Dialog>
  )
}
