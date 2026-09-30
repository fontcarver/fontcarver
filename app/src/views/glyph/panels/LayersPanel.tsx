import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { MasterSummary } from '../../../api'
import { useEditorStore } from '../../../stores/editorStore'
import { commitField } from '../commitField'
import { backgroundLayer } from '../glyphScene'
import { selectionToBackgroundCommand } from '../layerWorkflows'
import { runCommand } from '../../../shell/commands'
import { LayerManagementControls } from './LayerManagementControls'

import { LayerList } from './LayerList'
import { LayerColorField } from './LayerColorField'
import { LayerEffectsEditor } from './LayerEffectsEditor'
import { SmartGlyphEditor } from './SmartGlyphEditor'
import { SourceLayerEditor } from './SourceLayerEditor'
import styles from './panels.module.css'
import type { PanelContext } from './panelContext'
import {
  addLayerCommand,
  copyLayerCommand,
  removeLayerCommand,
  swapWithBackgroundCommand,
} from './panelCommands'

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
  scene,
  master,
  masters,
  reload,
}: LayersPanelProps) {
  const { t } = useTranslation()
  const overlays = useEditorStore((state) => state.overlays)
  const toggleOverlay = useEditorStore((state) => state.toggleOverlay)
  const background = backgroundLayer(glyph, master)
  const [effectsEditor, setEffectsEditor] = useState(false)
  const [smartEditor, setSmartEditor] = useState(false)
  const [sourceEditor, setSourceEditor] = useState<'copy' | 'edit' | null>(null)
  const selection = useEditorStore((state) => state.selection)
  const selectionCommand = master
    ? selectionToBackgroundCommand(target, master, selection)
    : null
  const current = glyph.layers.find((layer) => layer.id === outline.layer)

  const run = (
    command: Parameters<typeof commitField>[1],
    label: string,
  ): void => commitField(target, command, label, reload)

  return (
    <>
      {effectsEditor && current && (
        <LayerEffectsEditor
          doc={target.doc}
          glyph={glyph}
          layer={current}
          upem={scene.metrics.upem}
          onClose={() => setEffectsEditor(false)}
          onApplied={reload}
        />
      )}
      {smartEditor && (
        <SmartGlyphEditor
          doc={target.doc}
          glyph={glyph}
          onClose={() => setSmartEditor(false)}
          onApplied={reload}
        />
      )}
      {sourceEditor && current && master && (
        <SourceLayerEditor
          doc={target.doc}
          glyph={glyph.name}
          source={current}
          smartAxes={glyph.smart_axes}
          master={master}
          duplicate={sourceEditor === 'copy'}
          onClose={() => setSourceEditor(null)}
          onApplied={reload}
        />
      )}
      <LayerList
        doc={target.doc}
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

      {current && (
        <LayerManagementControls
          key={`${current.id}:${current.name}`}
          target={target}
          glyph={glyph}
          current={current}
          reload={reload}
        />
      )}
      {current && (
        <LayerColorField target={target} layer={current} reload={reload} />
      )}

      <div className={styles.actions}>
        <button
          type="button"
          className={styles.actionButton}
          disabled={!current || glyph.locked}
          onClick={() => setEffectsEditor(true)}
        >
          {t('layerEffects.title')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          onClick={() => setSmartEditor(true)}
        >
          {t('variation.smartAxes')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={!master || !current}
          onClick={() => setSourceEditor('copy')}
        >
          {t('variation.addSource')}
        </button>
        {current &&
          ('Brace' in current.kind ||
            'Bracket' in current.kind ||
            'Smart' in current.kind) && (
            <button
              type="button"
              className={styles.actionButton}
              onClick={() => setSourceEditor('edit')}
            >
              {t('variation.editSource')}
            </button>
          )}
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
          className={styles.actionButton}
          disabled={master === null || (background === null && glyph.locked)}
          onClick={() => runCommand('layer.editBackground')}
        >
          {t('panel.layers.editBackground')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={
            !selectionCommand || glyph.locked || current?.id === background?.id
          }
          title={t('panel.layers.selectionToBackgroundHint')}
          onClick={() => {
            if (selectionCommand)
              run(selectionCommand, t('editor.undo.selectionToBackground'))
          }}
        >
          {t('panel.layers.selectionToBackground')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={master === null || background === null || glyph.locked}
          title={t('panel.layers.clearBackgroundHint')}
          onClick={() => runCommand('layer.clearBackground')}
        >
          {t('panel.layers.clearBackground')}
        </button>
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
