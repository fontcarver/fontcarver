import { useTranslation } from 'react-i18next'

import type { MasterSummary } from '../../../api'
import { useEditorStore } from '../../../stores/editorStore'
import { commitField } from '../commitField'

import { LayerList } from './LayerList'
import styles from './panels.module.css'
import type { PanelContext } from './panelContext'
import {
  addLayerCommand,
  copyLayerCommand,
  removeLayerCommand,
  swapWithBackgroundCommand,
} from './panelCommands'
import { backgroundOf } from './layerRows'

export interface LayersPanelProps extends PanelContext {
  masters: readonly MasterSummary[]
}

/**
 * The Layers section (design spec §7.3).
 *
 * Every action is one command: a background is added or copied into, swapped
 * with the master layer, backed up with a timestamp, or removed. `RemoveLayer`
 * refuses master layers in the kernel, so the panel never offers it for one.
 */
export function LayersPanel({
  target,
  glyph,
  outline,
  master,
  masters,
  reload,
}: LayersPanelProps) {
  const { t } = useTranslation()
  const overlays = useEditorStore((state) => state.overlays)
  const toggleOverlay = useEditorStore((state) => state.toggleOverlay)
  const background = backgroundOf(glyph.layers, master)

  const run = (
    command: Parameters<typeof commitField>[1],
    label: string,
  ): void => commitField(target, command, label, reload)

  return (
    <>
      <LayerList
        layers={glyph.layers}
        masters={masters}
        current={outline.layer}
        rowActions={(row) =>
          row.removable ? (
            <button
              type="button"
              className={styles.chipRemove}
              aria-label={t('panel.layers.remove', {
                name: row.layer.name || row.layer.id,
              })}
              onClick={() =>
                run(
                  removeLayerCommand(glyph.name, row.layer.id),
                  t('editor.undo.removeLayer'),
                )
              }
            >
              ×
            </button>
          ) : null
        }
      />

      <div className={styles.actions}>
        <button
          type="button"
          className={styles.actionButton}
          disabled={master === null || background !== null}
          onClick={() => {
            if (master === null) return
            run(
              addLayerCommand(
                glyph.name,
                { Background: master },
                t('glyphInspector.layerKind.background'),
                outline.width,
              ),
              t('editor.undo.addLayer'),
            )
          }}
        >
          {t('panel.layers.addBackground')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={master === null}
          onClick={() => {
            if (master === null) return
            run(
              copyLayerCommand(glyph.name, outline.layer, {
                Background: master,
              }),
              t('editor.undo.copyToBackground'),
            )
          }}
        >
          {t('panel.layers.copyToBackground')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={master === null || background === null}
          onClick={() => {
            if (master === null) return
            run(
              swapWithBackgroundCommand(glyph.name, master),
              t('editor.undo.swapWithBackground'),
            )
          }}
        >
          {t('panel.layers.swapWithBackground')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={master === null}
          onClick={() => {
            if (master === null) return
            run(
              copyLayerCommand(glyph.name, outline.layer, {
                Backup: {
                  of: master,
                  time: Math.floor(Date.now() / 1000),
                },
              }),
              t('editor.undo.addBackup'),
            )
          }}
        >
          {t('panel.layers.addBackup')}
        </button>
      </div>

      <div className={styles.actions}>
        <button
          type="button"
          className={styles.toggle}
          aria-pressed={overlays.background}
          onClick={() => toggleOverlay('background')}
        >
          {t('panel.layers.showBackground')}
        </button>
        <button
          type="button"
          className={styles.toggle}
          aria-pressed={overlays.otherMasters}
          onClick={() => toggleOverlay('otherMasters')}
        >
          {t('panel.layers.showOtherMasters')}
        </button>
      </div>
    </>
  )
}
