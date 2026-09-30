import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { useEditorStore } from '../stores/editorStore'
import type { FilterTarget } from './contracts'
import { useFilterPreview } from './filterSession'
import { FilterPreviewGraphic } from './FilterPreviewGraphic'
import styles from './plugins.module.css'

/** Numeric Translate parameters and explicit Preview, Cancel and Apply actions. */
export function FilterControls({ pluginId }: { pluginId: string }) {
  const { t } = useTranslation()
  const active = useEditorStore((state) => state.activeGlyph)
  const phase = useFilterPreview((state) => state.phase)
  const preview = useFilterPreview((state) => state.preview)
  const error = useFilterPreview((state) => state.error)
  const [dx, setDx] = useState('20')
  const [dy, setDy] = useState('0')
  const layer = active?.scene.layer
  const target: FilterTarget | null =
    active?.outline &&
    layer &&
    'Master' in layer.kind &&
    layer.id === active.outline.layer
      ? {
          doc: active.doc,
          gid: active.gid,
          glyph: active.glyph.name,
          layer: layer.id,
          master: layer.kind.Master,
        }
      : null
  const identity = target
    ? `${target.doc}:${target.gid}:${target.layer}:${target.master}`
    : ''
  const busy = phase === 'loading' || phase === 'applying'
  const invalid =
    [dx, dy].some(
      (value) =>
        value.trim() === '' ||
        !Number.isFinite(Number(value)) ||
        Math.abs(Number(value)) > 32767,
    ) ||
    (Number(dx) === 0 && Number(dy) === 0)

  // Leaving a glyph, disabling the filter or closing the manager releases the preview.
  useEffect(
    () => () => useFilterPreview.getState().cancel(),
    [identity, pluginId],
  )

  const update = (axis: 'x' | 'y', value: string) => {
    useFilterPreview.getState().cancel()
    if (axis === 'x') setDx(value)
    else setDy(value)
  }
  const prepare = () => {
    if (!target || invalid) return
    void useFilterPreview
      .getState()
      .prepare(
        pluginId,
        target,
        { dx: Number(dx), dy: Number(dy) },
        t('plugins.translateUndo'),
      )
  }
  const apply = async () => {
    if (await useFilterPreview.getState().apply()) active?.reload()
  }
  return (
    <div className={styles.filter}>
      <p className={styles.help}>{t('plugins.filterScope')}</p>
      {target ? (
        <p>{t('plugins.target', { name: target.glyph })}</p>
      ) : (
        <p role="status">{t('plugins.openMaster')}</p>
      )}
      <div className={styles.parameters}>
        <label>
          {t('plugins.dx')}
          <input
            type="number"
            min={-32767}
            max={32767}
            step="any"
            value={dx}
            disabled={phase === 'applying'}
            onChange={(event) => update('x', event.target.value)}
          />
        </label>
        <label>
          {t('plugins.dy')}
          <input
            type="number"
            min={-32767}
            max={32767}
            step="any"
            value={dy}
            disabled={phase === 'applying'}
            onChange={(event) => update('y', event.target.value)}
          />
        </label>
      </div>
      {invalid && <p role="status">{t('plugins.invalidOffset')}</p>}
      <div className={styles.actions}>
        <button
          type="button"
          onClick={prepare}
          disabled={!target || invalid || busy}
        >
          {t(phase === 'loading' ? 'plugins.previewing' : 'plugins.preview')}
        </button>
        <button
          type="button"
          onClick={() => useFilterPreview.getState().cancel()}
          disabled={phase === 'idle' || phase === 'applying'}
        >
          {t('plugins.cancel')}
        </button>
        <button
          type="button"
          onClick={() => void apply()}
          disabled={phase !== 'ready' || preview?.pluginId !== pluginId}
        >
          {t(phase === 'applying' ? 'plugins.applying' : 'plugins.apply')}
        </button>
      </div>
      {error && (
        <p className={styles.error} role="alert">
          {t(error.key, { message: error.detail })}
        </p>
      )}
      {preview?.pluginId === pluginId && (
        <div aria-live="polite">
          <p>
            {t('plugins.previewSummary', {
              revision: preview.generation,
              layers: preview.result.layer_count,
            })}
          </p>
          {preview.result.decomposed_layers > 0 && (
            <p>
              {t('plugins.decomposed', {
                count: preview.result.decomposed_layers,
              })}
            </p>
          )}
          {preview.result.approximated_layers > 0 && (
            <p>
              {t('plugins.approximated', {
                count: preview.result.approximated_layers,
              })}
            </p>
          )}
          {preview.result.examples[0] && (
            <FilterPreviewGraphic example={preview.result.examples[0]} />
          )}
        </div>
      )}
    </div>
  )
}
