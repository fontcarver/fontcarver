import { useTranslation } from 'react-i18next'

import type { Layer, LayerKind } from '../../api'
import { Field } from '../../components/Field'
import { Panel } from '../../components/Panel'
import { useEditorStore } from '../../stores/editorStore'
import { colorLabelKey, colorLabelSwatch } from '../font/colorLabels'

import { formatUnicodes, pointCount } from './glyphScene'
import styles from './glyphView.module.css'

/** The i18n key naming a layer's role. */
function layerKindKey(kind: LayerKind): string {
  if ('Master' in kind) return 'master'
  if ('Brace' in kind) return 'brace'
  if ('Bracket' in kind) return 'bracket'
  if ('Background' in kind) return 'background'
  if ('Backup' in kind) return 'backup'
  return 'custom'
}

function layerLabel(layer: Layer, index: number): string {
  return layer.name || `#${index + 1}`
}

function round(value: number): string {
  return String(Math.round(value))
}

interface ListProps {
  empty: string
  children: React.ReactNode
  count: number
}

function List({ empty, count, children }: ListProps) {
  if (count === 0) return <p className={styles.inspectorEmpty}>{empty}</p>
  return <div className={styles.inspectorList}>{children}</div>
}

/**
 * The right pane on a Glyph tab: identity, metrics, components, anchors and
 * layers of the glyph on screen. Every field is read-only in P0; editing
 * arrives with the drawing tools.
 */
export function GlyphInspector() {
  const { t } = useTranslation()
  const active = useEditorStore((state) => state.activeGlyph)

  if (!active)
    return <p className={styles.inspectorEmpty}>{t('glyphView.loading')}</p>

  const { glyph, scene } = active
  const { metrics } = scene
  const swatch = colorLabelSwatch(glyph.color_label)

  return (
    <>
      <Panel title={t('glyphInspector.glyph')}>
        <Field label={t('glyphInspector.name')} readOnly>
          {glyph.name.toString()}
        </Field>
        <Field label={t('glyphInspector.unicodes')} readOnly>
          {formatUnicodes(glyph.unicodes)}
        </Field>
        <Field label={t('glyphInspector.production')} readOnly>
          {glyph.production_name ?? '—'}
        </Field>
        <Field label={t('glyphInspector.category')} readOnly>
          {glyph.category}
        </Field>
        <Field label={t('glyphInspector.subCategory')} readOnly>
          {glyph.sub_category}
        </Field>
        <Field label={t('glyphInspector.script')} readOnly>
          {glyph.script}
        </Field>
        <Field label={t('glyphInspector.export')} readOnly>
          {glyph.export ? t('common.yes') : t('common.no')}
        </Field>
        <Field label={t('glyphInspector.color')} readOnly>
          {glyph.color_label === null ? (
            '—'
          ) : (
            <>
              {swatch ? (
                <span
                  className={styles.swatch}
                  style={{ background: swatch }}
                  aria-hidden="true"
                />
              ) : null}
              {t(`fontView.color.${colorLabelKey(glyph.color_label)}`)}
            </>
          )}
        </Field>
        <Field label={t('glyphInspector.tags')} readOnly>
          {glyph.tags.join(', ') || '—'}
        </Field>
        <Field label={t('glyphInspector.kernGroups')} readOnly>
          {`${glyph.kern_group_side1 ?? '—'} / ${glyph.kern_group_side2 ?? '—'}`}
        </Field>
        <Field label={t('glyphInspector.note')} readOnly>
          {glyph.note || '—'}
        </Field>
      </Panel>

      <Panel title={t('glyphInspector.metrics')}>
        <Field label={t('glyphInspector.layer')} readOnly>
          {scene.layerName || '—'}
        </Field>
        <Field label={t('glyphView.metric.advance')} readOnly>
          {round(metrics.width)}
        </Field>
        <Field label={t('glyphView.metric.lsb')} readOnly>
          {metrics.lsb === null ? '—' : round(metrics.lsb)}
        </Field>
        <Field label={t('glyphView.metric.rsb')} readOnly>
          {metrics.rsb === null ? '—' : round(metrics.rsb)}
        </Field>
        <Field label={t('glyphInspector.bbox')} readOnly>
          {scene.bbox
            ? `${round(scene.bbox.x0)} ${round(scene.bbox.y0)} → ${round(
                scene.bbox.x1,
              )} ${round(scene.bbox.y1)}`
            : '—'}
        </Field>
        <Field label={t('glyphInspector.points')} readOnly>
          {pointCount(scene)}
        </Field>
        {metrics.verticalOrigin === null ? null : (
          <Field label={t('glyphView.metric.verticalOrigin')} readOnly>
            {round(metrics.verticalOrigin)}
          </Field>
        )}
        {metrics.verticalWidth === null ? null : (
          <Field label={t('glyphView.metric.verticalAdvance')} readOnly>
            {round(metrics.verticalWidth)}
          </Field>
        )}
      </Panel>

      <Panel title={t('glyphInspector.components')}>
        <List
          count={scene.components.length}
          empty={t('glyphInspector.noComponents')}
        >
          {scene.components.map((component, index) => (
            <div
              key={`${component.base.toString()}-${index}`}
              className={styles.inspectorRow}
            >
              <span className={styles.inspectorName}>
                {component.base.toString()}
              </span>
              <span>
                {`${round(component.transform[4])}, ${round(component.transform[5])}`}
              </span>
            </div>
          ))}
        </List>
      </Panel>

      <Panel title={t('glyphInspector.anchors')}>
        <List
          count={scene.anchors.length}
          empty={t('glyphInspector.noAnchors')}
        >
          {scene.anchors.map((anchor, index) => (
            <div
              key={`${anchor.name}-${index}`}
              className={styles.inspectorRow}
            >
              <span className={styles.inspectorName}>{anchor.name}</span>
              <span>{`${round(anchor.x)}, ${round(anchor.y)}`}</span>
            </div>
          ))}
        </List>
      </Panel>

      <Panel title={t('glyphInspector.layers')}>
        <List count={glyph.layers.length} empty={t('glyphInspector.noLayers')}>
          {glyph.layers.map((layer, index) => (
            <div key={layer.id.toString()} className={styles.inspectorRow}>
              <span className={styles.inspectorName}>
                {layerLabel(layer, index)}
              </span>
              <span>
                {t(`glyphInspector.layerKind.${layerKindKey(layer.kind)}`)}
              </span>
            </div>
          ))}
        </List>
      </Panel>
    </>
  )
}
