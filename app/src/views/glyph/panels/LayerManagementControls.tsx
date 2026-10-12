import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Layer } from '../../../api'
import { commitField } from '../commitField'
import { duplicateCustomLayerCommand } from '../layerWorkflows'
import { addLayerCommand } from './panelCommands'
import type { PanelContext } from './panelContext'
import styles from './panels.module.css'

interface Props extends Pick<PanelContext, 'target' | 'glyph' | 'reload'> {
  current: Layer
}

/** Named custom layers and renaming use the same persistent command history. */
export function LayerManagementControls({
  target,
  glyph,
  current,
  reload,
}: Props) {
  const { t } = useTranslation()
  const [name, setName] = useState(
    current.name || t('panel.layers.defaultName'),
  )
  const trimmed = name.trim()
  const disabled = !trimmed || glyph.locked
  const run = (command: Parameters<typeof commitField>[1], label: string) =>
    commitField(target, command, label, reload)
  return (
    <>
      <label className={styles.row}>
        <span className={styles.rowLabel}>{t('panel.layers.name')}</span>
        <input
          className={styles.textInput}
          value={name}
          aria-label={t('panel.layers.name')}
          onChange={(event) => setName(event.currentTarget.value)}
        />
      </label>
      <div className={styles.actions}>
        <button
          type="button"
          className={styles.actionButton}
          disabled={disabled}
          onClick={() =>
            run(
              addLayerCommand(
                glyph.name,
                { Custom: trimmed },
                trimmed,
                current.width,
              ),
              t('editor.undo.addLayer'),
            )
          }
        >
          {t('panel.layers.addCustom')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={disabled}
          onClick={() =>
            run(
              duplicateCustomLayerCommand(glyph.name, current, trimmed),
              t('editor.undo.duplicateLayer'),
            )
          }
        >
          {t('panel.layers.duplicate')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={disabled || trimmed === current.name}
          onClick={() =>
            run(
              {
                RenameLayer: {
                  glyph: glyph.name,
                  layer: current.id,
                  name: trimmed,
                },
              },
              t('editor.undo.renameLayer'),
            )
          }
        >
          {t('panel.layers.rename')}
        </button>
      </div>
    </>
  )
}
