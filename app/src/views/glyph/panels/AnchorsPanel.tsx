import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Select } from '../../../components/Select'
import { activeEditor } from '../../../editor/activeEditor'
import { setAnchorCommand } from '../../../editor/pointCommands'
import { selectionOf } from '../../../editor/selection'
import { useEditorStore } from '../../../stores/editorStore'
import { NumberField } from '../../../components/NumberField'
import { commitField } from '../commitField'

import styles from './panels.module.css'
import type { PanelContext } from './panelContext'
import {
  addAnchorCommand,
  ANCHOR_SUGGESTIONS,
  removeAnchorsCommand,
  suggestAnchorName,
} from './panelCommands'
import { TextRow } from './TextRow'

/**
 * The Anchors section (design spec §7.5): the layer's attachment points, an
 * add button that suggests the next conventional name, and a delete per row.
 * Clicking a row selects the anchor on the canvas, which is what makes it
 * draggable and gives the Info box its rows.
 */
export function AnchorsPanel({ target, outline, scene, reload }: PanelContext) {
  const { t } = useTranslation()
  const [pending, setPending] = useState<string | null>(null)
  const selection = useEditorStore((state) => state.selection)
  const anchors = outline.anchors
  // The suggestion follows the layer until the user picks another name.
  const suggested = pending ?? suggestAnchorName(anchors)

  const run = (command: Parameters<typeof commitField>[1], label: string) =>
    commitField(target, command, label, reload)

  return (
    <>
      {anchors.length === 0 ? (
        <p className={styles.empty}>{t('glyphInspector.noAnchors')}</p>
      ) : (
        <div className={styles.list}>
          {anchors.map((anchor, index) => {
            const selected = selection.has(`anchor:${index}`)
            return (
              <div
                key={`${anchor.name}-${index}`}
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
                      selectionOf([{ kind: 'anchor', index }]),
                    )
                  }
                >
                  {anchor.name}
                </button>
                <span className={styles.rowMeta}>
                  {`${Math.round(anchor.x)}, ${Math.round(anchor.y)}`}
                </span>
                <button
                  type="button"
                  className={styles.chipRemove}
                  aria-label={t('panel.anchors.remove', { name: anchor.name })}
                  onClick={() =>
                    run(
                      removeAnchorsCommand(target, [index]),
                      t('editor.undo.removeAnchor'),
                    )
                  }
                >
                  ×
                </button>
                {selected ? (
                  <div className={styles.rowFields}>
                    <TextRow
                      label={t('infoBox.name')}
                      value={anchor.name}
                      required
                      onCommit={(name) =>
                        run(
                          setAnchorCommand(target, index, { ...anchor, name }),
                          t('editor.undo.setAnchor'),
                        )
                      }
                    />
                    <span />
                    <NumberField
                      label={t('infoBox.x')}
                      value={anchor.x}
                      invalidHint={t('infoBox.invalid')}
                      onCommit={(x) =>
                        run(
                          setAnchorCommand(target, index, { ...anchor, x }),
                          t('editor.undo.setAnchor'),
                        )
                      }
                    />
                    <NumberField
                      label={t('infoBox.y')}
                      value={anchor.y}
                      invalidHint={t('infoBox.invalid')}
                      onCommit={(y) =>
                        run(
                          setAnchorCommand(target, index, { ...anchor, y }),
                          t('editor.undo.setAnchor'),
                        )
                      }
                    />
                  </div>
                ) : null}
              </div>
            )
          })}
        </div>
      )}

      <div className={styles.actions}>
        <Select
          label={t('panel.anchors.suggestion')}
          value={suggested}
          options={[...new Set([...ANCHOR_SUGGESTIONS, suggested])].map(
            (name) => ({ value: name, label: name }),
          )}
          onChange={setPending}
        />
        <button
          type="button"
          className={styles.actionButton}
          onClick={() =>
            run(
              addAnchorCommand(target, {
                name: suggested,
                // A new anchor lands on the horizontal centre of the advance,
                // at the x-height, which is where `top` belongs in Latin and a
                // sensible starting point everywhere else.
                x: Math.round(outline.width / 2),
                y: Math.round(scene.metrics.xHeight),
              }),
              t('editor.undo.addAnchor'),
            )
          }
        >
          {t('panel.anchors.add')}
        </button>
      </div>
      <p className={styles.hint}>{t('panel.anchors.hint')}</p>
    </>
  )
}
