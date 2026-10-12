import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { StrokeCap, StrokeJoin, StrokeOptions } from '../api'
import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { Select } from '../components/Select'
import { i18n } from '../i18n'
import { useUiStore } from '../stores/uiStore'
import { pathContext, runOnLayer } from '../views/glyph/pathCommands'
import { DEFAULT_STROKE, pathOpCommand } from '../views/glyph/pathOps'

import styles from './dialogs.module.css'
import { NumberField } from './NumberField'

const CAPS: readonly StrokeCap[] = ['Butt', 'Square', 'Round']
const JOINS: readonly StrokeJoin[] = ['Bevel', 'Miter', 'Round']

/**
 * Expand Stroke (design spec §11): the `StrokeOptions` `fc-geom` takes. The
 * calligraphic nib has three numbers of its own and belongs with the drawing
 * tools, so this dialog offers the round and rectangular pens only.
 */
export function ExpandStrokeDialog() {
  const { t } = useTranslation()
  const close = () => useUiStore.getState().setDialog(null)
  const [options, setOptions] = useState<StrokeOptions>(DEFAULT_STROKE)
  const set = (patch: Partial<StrokeOptions>) =>
    setOptions((current) => ({ ...current, ...patch }))

  const confirm = () => {
    const context = pathContext()
    if (context)
      runOnLayer(
        pathOpCommand(context.target, { ExpandStroke: options }),
        i18n.t('editor.undo.expandStroke'),
      )
    close()
  }

  return (
    <Dialog
      title={t('dialog.expandStroke.title')}
      size="sm"
      onCancel={close}
      onConfirm={confirm}
      footer={
        <>
          <Button onClick={close}>{t('common.cancel')}</Button>
          <Button variant="primary" onClick={confirm}>
            {t('dialog.expandStroke.run')}
          </Button>
        </>
      }
    >
      <div className={styles.form}>
        <div className={styles.row}>
          <span className={styles.rowLabel}>
            {t('dialog.expandStroke.width')}
          </span>
          <NumberField
            label={t('dialog.expandStroke.width')}
            value={options.width}
            min={1}
            onCommit={(width) => set({ width })}
          />
        </div>
        <div className={styles.row}>
          <span className={styles.rowLabel}>
            {t('dialog.expandStroke.nib')}
          </span>
          <Select
            label={t('dialog.expandStroke.nib')}
            value={options.nib === 'Rect' ? 'Rect' : 'Round'}
            options={[
              { value: 'Round', label: t('dialog.expandStroke.nibRound') },
              { value: 'Rect', label: t('dialog.expandStroke.nibRect') },
            ]}
            onChange={(nib) => set({ nib: nib as 'Round' | 'Rect' })}
          />
        </div>
        <div className={styles.row}>
          <span className={styles.rowLabel}>
            {t('dialog.expandStroke.cap')}
          </span>
          <Select
            label={t('dialog.expandStroke.cap')}
            value={options.cap}
            options={CAPS.map((cap) => ({
              value: cap,
              label: t(`dialog.expandStroke.cap${cap}`),
            }))}
            onChange={(cap) => set({ cap })}
          />
        </div>
        <div className={styles.row}>
          <span className={styles.rowLabel}>
            {t('dialog.expandStroke.join')}
          </span>
          <Select
            label={t('dialog.expandStroke.join')}
            value={options.join}
            options={JOINS.map((join) => ({
              value: join,
              label: t(`dialog.expandStroke.join${join}`),
            }))}
            onChange={(join) => set({ join })}
          />
        </div>
        <div className={styles.row}>
          <span className={styles.rowLabel}>
            {t('dialog.expandStroke.miterLimit')}
          </span>
          <NumberField
            label={t('dialog.expandStroke.miterLimit')}
            value={options.miter_limit}
            min={1}
            onCommit={(miter_limit) => set({ miter_limit })}
          />
        </div>
        <div className={styles.checkRow}>
          <input
            id="stroke-remove-internal"
            type="checkbox"
            checked={options.remove_internal}
            onChange={(event) => set({ remove_internal: event.target.checked })}
          />
          <label htmlFor="stroke-remove-internal" className={styles.checkLabel}>
            {t('dialog.expandStroke.removeInternal')}
          </label>
        </div>
        <div className={styles.checkRow}>
          <input
            id="stroke-remove-external"
            type="checkbox"
            checked={options.remove_external}
            onChange={(event) => set({ remove_external: event.target.checked })}
          />
          <label htmlFor="stroke-remove-external" className={styles.checkLabel}>
            {t('dialog.expandStroke.removeExternal')}
          </label>
        </div>
      </div>
    </Dialog>
  )
}
