import { useTranslation } from 'react-i18next'
import { Lock, LockOpen } from 'lucide-react'

import type { Guide } from '../../../api'
import { NumberField } from '../../../components/NumberField'
import { activeEditor } from '../../../editor/activeEditor'
import { setGuideCommand } from '../../../editor/pointCommands'
import { selectionOf, type GuideScope } from '../../../editor/selection'
import { useEditorStore } from '../../../stores/editorStore'
import { commitField } from '../commitField'
import { GuideAnchorRow } from '../GuideAnchorRow'
import { inkEdgesOf } from '../guideAnchor'
import { measurementGuideKey } from '../guideIdentity'

import styles from './panels.module.css'
import type { PanelContext } from './panelContext'
import { removeGuideCommand } from './panelCommands'
import { TextRow } from './TextRow'

export interface GuideRowsProps {
  context: PanelContext
  scope: GuideScope
  guides: readonly Guide[]
}

/**
 * One scope's guide list (design spec §7.6). Layer guides belong to this glyph
 * alone; master guides are the font-wide "global" set the canvas draws in the
 * global colour. Both are edited through the same `SetGuide`, differing only
 * in the scope the command names.
 */
export function GuideRows({ context, scope, guides }: GuideRowsProps) {
  const { t } = useTranslation()
  const { target, master, reload, scene } = context
  const edges = inkEdgesOf(scene.metrics)
  const selection = useEditorStore((state) => state.selection)
  const measurement = useEditorStore((state) => state.measurementGuides)
  const toggleMeasurement = useEditorStore(
    (state) => state.toggleMeasurementGuide,
  )

  if (guides.length === 0)
    return <p className={styles.empty}>{t('panel.guides.none')}</p>

  const write = (index: number, guide: Guide) =>
    commitField(
      target,
      setGuideCommand(target, scope, master, index, guide),
      t('editor.undo.setGuide'),
      reload,
    )

  return (
    <div className={styles.list}>
      {guides.map((guide, index) => {
        const key = `${scope}:${index}`
        const measurementKey = measurementGuideKey(
          { doc: target.doc, layer: target.layer, master },
          scope,
          index,
        )
        const selected = selection.has(`guide:${key}`)
        return (
          <div
            key={key}
            className={`${styles.listRow} ${
              selected ? styles.listRowActive : ''
            }`}
          >
            <button
              type="button"
              className={styles.rowName}
              aria-current={selected}
              onClick={() =>
                activeEditor()?.setSelection(
                  selectionOf([{ kind: 'guide', index, scope }]),
                )
              }
            >
              {guide.name ||
                t('panel.guides.unnamed', {
                  x: Math.round(guide.x),
                  y: Math.round(guide.y),
                })}
            </button>
            <span className={styles.rowMeta}>{`${Math.round(
              guide.angle,
            )}°`}</span>
            <button
              type="button"
              className={styles.chipRemove}
              aria-label={t('panel.guides.lock')}
              aria-pressed={guide.locked}
              onClick={() => write(index, { ...guide, locked: !guide.locked })}
            >
              {guide.locked ? (
                <Lock aria-hidden="true" size={14} />
              ) : (
                <LockOpen aria-hidden="true" size={14} />
              )}
            </button>
            <button
              type="button"
              className={styles.chipRemove}
              aria-label={t('panel.guides.measurement')}
              aria-pressed={measurement.has(measurementKey)}
              onClick={() => toggleMeasurement(measurementKey)}
            >
              ⟺
            </button>
            <button
              type="button"
              className={styles.chipRemove}
              aria-label={t('panel.guides.remove')}
              onClick={() =>
                commitField(
                  target,
                  removeGuideCommand(target, scope, master, index),
                  t('editor.undo.removeGuide'),
                  reload,
                )
              }
            >
              ×
            </button>
            {selected ? (
              <div className={styles.rowFields}>
                <NumberField
                  label={t('infoBox.x')}
                  value={guide.x}
                  invalidHint={t('infoBox.invalid')}
                  onCommit={(x) => write(index, { ...guide, x })}
                />
                <NumberField
                  label={t('infoBox.y')}
                  value={guide.y}
                  invalidHint={t('infoBox.invalid')}
                  onCommit={(y) => write(index, { ...guide, y })}
                />
                <NumberField
                  label={t('infoBox.angle')}
                  value={guide.angle}
                  invalidHint={t('infoBox.invalid')}
                  onCommit={(angle) => write(index, { ...guide, angle })}
                />
                <span />
                <TextRow
                  label={t('infoBox.name')}
                  value={guide.name ?? ''}
                  onCommit={(name) =>
                    write(index, { ...guide, name: name === '' ? null : name })
                  }
                />
                <GuideAnchorRow
                  guide={guide}
                  edges={edges}
                  onChange={(next) => write(index, { ...guide, ...next })}
                />
              </div>
            ) : null}
          </div>
        )
      })}
    </div>
  )
}
