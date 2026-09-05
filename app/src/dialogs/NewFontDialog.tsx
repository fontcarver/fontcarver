import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { useUiStore } from '../stores/uiStore'

import styles from './dialogs.module.css'
import {
  CJK_UNITS_PER_EM,
  UNITS_PER_EM_PRESETS,
  createFont,
  type MetricsPreset,
  type StarterSet,
} from './newFont'

const PRESETS: readonly MetricsPreset[] = ['latin', 'cjk']
const STARTERS: readonly StarterSet[] = [
  'none',
  'basicLatin',
  'latin1',
  'gb2312',
]

/**
 * New Font: family, style, units per em, a metrics preset and a starter
 * repertoire. Creating runs `session.new`, one `doc.apply` batch and, when a
 * starter set is chosen, `glyphs.add`; the document opens in a Font tab.
 */
export function NewFontDialog() {
  const { t } = useTranslation()
  const setDialog = useUiStore((state) => state.setDialog)
  const [family, setFamily] = useState(() => t('document.newFamily'))
  const [style, setStyle] = useState('Regular')
  const [unitsPerEm, setUnitsPerEm] = useState(1000)
  const [custom, setCustom] = useState(false)
  const [preset, setPreset] = useState<MetricsPreset>('latin')
  const [starter, setStarter] = useState<StarterSet>('basicLatin')
  const [busy, setBusy] = useState(false)

  const close = () => setDialog(null)
  const valid =
    family.trim().length > 0 && unitsPerEm >= 16 && unitsPerEm <= 16_384

  const choosePreset = (next: MetricsPreset) => {
    setPreset(next)
    // The CJK preset's 880 / −120 only describes a 1000-unit em.
    if (next === 'cjk') {
      setUnitsPerEm(CJK_UNITS_PER_EM)
      setCustom(false)
    }
  }

  const create = () => {
    if (!valid || busy) return
    setBusy(true)
    void createFont({
      family: family.trim(),
      style: style.trim(),
      unitsPerEm,
      preset,
      starter,
    })
      .then(close)
      .catch((error: unknown) => {
        setBusy(false)
        useUiStore.getState().pushToast({
          kind: 'error',
          message: error instanceof Error ? error.message : String(error),
        })
      })
  }

  return (
    <Dialog
      title={t('dialog.newFont.title')}
      size="sm"
      onCancel={close}
      onConfirm={create}
      footer={
        <>
          <Button onClick={close}>{t('common.cancel')}</Button>
          <Button variant="primary" disabled={!valid || busy} onClick={create}>
            {t('dialog.newFont.create')}
          </Button>
        </>
      }
    >
      <div className={styles.form}>
        <label className={styles.row}>
          <span className={styles.rowLabel}>{t('dialog.newFont.family')}</span>
          <input
            autoFocus
            className={styles.text}
            value={family}
            onChange={(event) => setFamily(event.target.value)}
          />
        </label>
        <label className={styles.row}>
          <span className={styles.rowLabel}>{t('dialog.newFont.style')}</span>
          <input
            className={styles.text}
            value={style}
            onChange={(event) => setStyle(event.target.value)}
          />
        </label>
        <div className={styles.row}>
          <span className={styles.rowLabel}>
            {t('dialog.newFont.unitsPerEm')}
          </span>
          <div className={styles.choices}>
            {UNITS_PER_EM_PRESETS.map((value) => (
              <label key={value} className={styles.choice}>
                <input
                  type="radio"
                  name="units-per-em"
                  checked={!custom && unitsPerEm === value}
                  onChange={() => {
                    setCustom(false)
                    setUnitsPerEm(value)
                  }}
                />
                {value}
              </label>
            ))}
            <label className={styles.choice}>
              <input
                type="radio"
                name="units-per-em"
                checked={custom}
                onChange={() => setCustom(true)}
              />
              {t('dialog.newFont.custom')}
            </label>
            {custom ? (
              <input
                type="number"
                min={16}
                max={16384}
                aria-label={t('dialog.newFont.customUnitsPerEm')}
                className={styles.number}
                value={unitsPerEm}
                onChange={(event) => setUnitsPerEm(Number(event.target.value))}
              />
            ) : null}
          </div>
        </div>
        <div className={`${styles.row} ${styles.rowTop}`}>
          <span className={styles.rowLabel}>{t('dialog.newFont.metrics')}</span>
          <div className={styles.rowStack}>
            <div className={styles.choices}>
              {PRESETS.map((value) => (
                <label key={value} className={styles.choice}>
                  <input
                    type="radio"
                    name="metrics-preset"
                    checked={preset === value}
                    onChange={() => choosePreset(value)}
                  />
                  {t(`dialog.newFont.preset.${value}`)}
                </label>
              ))}
            </div>
            <p className={styles.hint}>
              {t(`dialog.newFont.presetHint.${preset}`)}
            </p>
          </div>
        </div>
        <div className={`${styles.row} ${styles.rowTop}`}>
          <span className={styles.rowLabel}>{t('dialog.newFont.starter')}</span>
          <div className={styles.rowStack}>
            <div className={styles.choices}>
              {STARTERS.map((value) => (
                <label key={value} className={styles.choice}>
                  <input
                    type="radio"
                    name="starter-set"
                    checked={starter === value}
                    onChange={() => setStarter(value)}
                  />
                  {t(`dialog.newFont.set.${value}`)}
                </label>
              ))}
            </div>
            <p className={styles.hint}>{t('dialog.newFont.starterHint')}</p>
          </div>
        </div>
      </div>
    </Dialog>
  )
}
