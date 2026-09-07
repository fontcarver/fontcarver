import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { SimplifyOptions } from '../api'
import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { i18n } from '../i18n'
import { useUiStore } from '../stores/uiStore'
import { runOnLayer } from '../views/glyph/pathCommands'
import { DEFAULT_SIMPLIFY, pathOpCommand } from '../views/glyph/pathOps'
import { pathContext } from '../views/glyph/pathCommands'

import styles from './dialogs.module.css'
import { NumberField } from './NumberField'

/** The boolean policies, in the order the dialog lists them. */
const FLAGS: readonly (keyof SimplifyOptions)[] = [
  'ignore_slopes',
  'ignore_extrema',
  'smooth_curves',
  'force_lines',
  'nearly_hv_lines',
  'start_at_extrema',
]

const FLAG_KEYS: Record<string, string> = {
  ignore_slopes: 'ignoreSlopes',
  ignore_extrema: 'ignoreExtrema',
  smooth_curves: 'smoothCurves',
  force_lines: 'forceLines',
  nearly_hv_lines: 'nearlyHvLines',
  start_at_extrema: 'startAtExtrema',
}

/**
 * Simplify (design spec §11): the `SimplifyOptions` the kernel's refitter
 * takes, with the kernel's own defaults. It is a whole-layer operation, so the
 * dialog says nothing about the selection.
 */
export function SimplifyDialog() {
  const { t } = useTranslation()
  const close = () => useUiStore.getState().setDialog(null)
  const [options, setOptions] = useState<SimplifyOptions>(DEFAULT_SIMPLIFY)

  const confirm = () => {
    const context = pathContext()
    if (context)
      runOnLayer(
        pathOpCommand(context.target, { Simplify: options }),
        i18n.t('editor.undo.simplify'),
      )
    close()
  }

  return (
    <Dialog
      title={t('dialog.simplify.title')}
      size="sm"
      onCancel={close}
      onConfirm={confirm}
      footer={
        <>
          <Button onClick={close}>{t('common.cancel')}</Button>
          <Button variant="primary" onClick={confirm}>
            {t('dialog.simplify.run')}
          </Button>
        </>
      }
    >
      <div className={styles.form}>
        <div className={styles.row}>
          <span className={styles.rowLabel}>{t('dialog.simplify.error')}</span>
          <div className={styles.rowStack}>
            <NumberField
              label={t('dialog.simplify.error')}
              value={options.error}
              min={0.001}
              step={0.1}
              onCommit={(error) =>
                setOptions((current) => ({ ...current, error }))
              }
            />
            <span className={styles.hint}>
              {t('dialog.simplify.errorHint')}
            </span>
          </div>
        </div>
        {FLAGS.map((flag) => (
          <div key={flag} className={styles.checkRow}>
            <input
              id={`simplify-${flag}`}
              type="checkbox"
              checked={options[flag] === true}
              onChange={(event) =>
                setOptions((current) => ({
                  ...current,
                  [flag]: event.target.checked,
                }))
              }
            />
            <label htmlFor={`simplify-${flag}`} className={styles.checkLabel}>
              {t(`dialog.simplify.${FLAG_KEYS[flag] ?? flag}`)}
            </label>
          </div>
        ))}
      </div>
    </Dialog>
  )
}
