import { useTranslation } from 'react-i18next'

import { activeEditor } from '../../../editor/activeEditor'
import { selectionOf } from '../../../editor/selection'
import { commitField } from '../commitField'

import { GuideRows } from './GuideRows'
import styles from './panels.module.css'
import type { PanelContext } from './panelContext'
import { addGuideCommand, newGuide } from './panelCommands'

/**
 * The Guides section (design spec §7.6): the layer's own guides and the
 * master's, each with an add button. A guide added here starts horizontal at
 * the pointer-free centre of the layer; ⌥-dragging from a ruler places one
 * exactly, and the canvas selects it either way.
 */
export function GuidesPanel(context: PanelContext) {
  const { t } = useTranslation()
  const { target, outline, master, masterGuides, scene, reload } = context

  const add = (scope: 'layer' | 'master', angle: number) => {
    const index =
      scope === 'layer' ? outline.guides.length : masterGuides.length
    commitField(
      target,
      addGuideCommand(
        target,
        scope,
        master,
        newGuide(
          angle === 0 ? 0 : Math.round(outline.width / 2),
          angle === 0 ? Math.round(scene.metrics.xHeight) : 0,
          angle,
        ),
      ),
      t('editor.undo.addGuide'),
      () => {
        reload()
        activeEditor()?.setSelection(
          selectionOf([{ kind: 'guide', index, scope }]),
        )
      },
    )
  }

  return (
    <>
      <p className={styles.hint}>{t('panel.guides.local')}</p>
      <GuideRows context={context} scope="layer" guides={outline.guides} />
      <div className={styles.actions}>
        <button
          type="button"
          className={styles.actionButton}
          onClick={() => add('layer', 0)}
        >
          {t('panel.guides.addHorizontal')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          onClick={() => add('layer', 90)}
        >
          {t('panel.guides.addVertical')}
        </button>
      </div>

      <p className={styles.hint}>{t('panel.guides.global')}</p>
      <GuideRows context={context} scope="master" guides={masterGuides} />
      <div className={styles.actions}>
        <button
          type="button"
          className={styles.actionButton}
          disabled={master === null}
          onClick={() => add('master', 0)}
        >
          {t('panel.guides.addHorizontal')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={master === null}
          onClick={() => add('master', 90)}
        >
          {t('panel.guides.addVertical')}
        </button>
      </div>
      <p className={styles.hint}>{t('panel.guides.rulerHint')}</p>
    </>
  )
}
