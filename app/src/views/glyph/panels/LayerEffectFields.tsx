import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type DocId,
  type Glyph,
  type LayerEffectKind,
} from '../../../api'
import { Button } from '../../../components/Button'
import { Select } from '../../../components/Select'
import styles from './layerEffects.module.css'

/** Parameters and stored-mask capture for one live layer operation. */
export function LayerEffectFields({
  effect,
  onChange,
  doc,
  glyph,
  onError,
  onBusy,
}: {
  effect: LayerEffectKind
  onChange: (effect: LayerEffectKind) => void
  doc: DocId
  glyph: Glyph
  onError: (error: unknown) => void
  onBusy: (busy: boolean) => void
}) {
  const { t } = useTranslation()
  const [maskGlyph, setMaskGlyph] = useState(glyph.name)
  const [maskLayer, setMaskLayer] = useState('')
  if (effect.kind === 'remove_overlap')
    return <p>{t('layerEffects.overlapHelp')}</p>
  if (effect.kind === 'stroke')
    return (
      <div className={styles.numbers}>
        <label>
          {t('layerEffects.nib')}
          <Select
            label={t('layerEffects.nib')}
            value={effect.nib}
            options={(['ellipse', 'rectangle'] as const).map((value) => ({
              value,
              label: t(`layerEffects.${value}`),
            }))}
            onChange={(nib) => onChange({ ...effect, nib })}
          />
        </label>
        {(['width', 'height', 'angle', 'miter_limit'] as const).map((key) => (
          <label key={key}>
            {t(`layerEffects.${key}`)}
            <input
              type="number"
              step={0.1}
              value={Number.isFinite(effect[key]) ? effect[key] : ''}
              onChange={(e) =>
                onChange({ ...effect, [key]: e.target.valueAsNumber })
              }
            />
          </label>
        ))}
        <label>
          {t('layerEffects.cap')}
          <Select
            label={t('layerEffects.cap')}
            value={effect.cap}
            options={(['round', 'butt', 'square'] as const).map((value) => ({
              value,
              label: t(`layerEffects.${value}`),
            }))}
            onChange={(cap) => onChange({ ...effect, cap })}
          />
        </label>
        <label>
          {t('layerEffects.join')}
          <Select
            label={t('layerEffects.join')}
            value={effect.join}
            options={(['round', 'bevel', 'miter'] as const).map((value) => ({
              value,
              label: t(`layerEffects.${value}`),
            }))}
            onChange={(join) => onChange({ ...effect, join })}
          />
        </label>
        <p className={styles.wide}>{t('layerEffects.penHelp')}</p>
      </div>
    )
  const capture = async () => {
    onBusy(true)
    try {
      const path = await call('effects.mask', {
        doc,
        glyph: maskGlyph,
        layer: maskLayer || null,
      })
      onChange({ ...effect, path })
    } catch (error) {
      onError(error)
    } finally {
      onBusy(false)
    }
  }
  return (
    <div className={styles.mask}>
      <label>
        {t('layerEffects.operation')}
        <Select
          label={t('layerEffects.operation')}
          value={effect.operation}
          options={(['union', 'intersect', 'subtract', 'xor'] as const).map(
            (value) => ({ value, label: t(`layerEffects.${value}`) }),
          )}
          onChange={(operation) => onChange({ ...effect, operation })}
        />
      </label>
      <label>
        {t('layerEffects.path')}
        <textarea
          rows={3}
          spellCheck={false}
          value={effect.path}
          onChange={(e) => onChange({ ...effect, path: e.target.value })}
        />
      </label>
      <div className={styles.row}>
        <label>
          {t('layerEffects.maskGlyph')}
          <input
            value={maskGlyph}
            onChange={(e) => {
              setMaskGlyph(e.target.value)
              setMaskLayer('')
            }}
          />
        </label>
        <Select
          label={t('layerEffects.maskLayer')}
          value={maskLayer}
          options={[
            { value: '', label: t('layerEffects.defaultMaster') },
            ...(maskGlyph === glyph.name
              ? glyph.layers.map((layer, index) => ({
                  value: layer.id,
                  label:
                    layer.name || `${t('layerEffects.layer')} ${index + 1}`,
                }))
              : []),
          ]}
          onChange={setMaskLayer}
        />
        <Button
          onClick={() => {
            void capture()
          }}
        >
          {t('layerEffects.capture')}
        </Button>
      </div>
      <p>{t('layerEffects.maskHelp')}</p>
    </div>
  )
}
