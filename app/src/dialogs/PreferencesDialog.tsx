import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { SnapPrefs } from '../api'
import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { SUPPORTED_LANGUAGES } from '../i18n'
import { usePrefsStore } from '../stores/prefsStore'
import { useUiStore, type ThemePreference } from '../stores/uiStore'

import styles from './dialogs.module.css'

/** Left-hand rail of the dialog, in the order the design spec lists it. */
const CATEGORIES = ['general', 'editor', 'snapping'] as const
type Category = (typeof CATEGORIES)[number]

const THEMES: readonly ThemePreference[] = ['system', 'light', 'dark']

const SNAP_FIELDS: readonly (keyof SnapPrefs)[] = [
  'nodes',
  'guides',
  'metrics',
  'grid',
  'components',
]

const NUDGE_STEPS = ['small', 'medium', 'large'] as const

/**
 * Preferences: General, Editor and Snapping. Every change is applied and
 * persisted through `prefs.set` immediately — the dialog has no OK button, only
 * a way out and a way back to the shipped defaults.
 */
export function PreferencesDialog() {
  const { t } = useTranslation()
  const setDialog = useUiStore((state) => state.setDialog)
  const prefs = usePrefsStore((state) => state.prefs)
  const loaded = usePrefsStore((state) => state.loaded)
  const update = usePrefsStore((state) => state.update)
  const reset = usePrefsStore((state) => state.reset)
  const [category, setCategory] = useState<Category>('general')

  useEffect(() => {
    if (!loaded) void usePrefsStore.getState().load()
  }, [loaded])

  const close = () => setDialog(null)

  const setNudge = (index: number, value: number) => {
    const nudge: [number, number, number] = [...prefs.nudge]
    nudge[index] = value
    void update({ nudge })
  }

  return (
    <Dialog
      title={t('dialog.prefs.title')}
      size="lg"
      scrollBody
      onCancel={close}
      footerSpread
      footer={
        <>
          <Button onClick={() => void reset()}>
            {t('dialog.prefs.reset')}
          </Button>
          <Button variant="primary" onClick={close}>
            {t('common.done')}
          </Button>
        </>
      }
    >
      <div className={styles.prefsLayout}>
        <div
          role="tablist"
          aria-label={t('dialog.prefs.categories')}
          aria-orientation="vertical"
          className={styles.prefsRail}
        >
          {CATEGORIES.map((entry) => (
            <button
              key={entry}
              type="button"
              role="tab"
              aria-selected={entry === category}
              className={
                entry === category
                  ? `${styles.prefsCategory} ${styles.prefsCategoryActive}`
                  : styles.prefsCategory
              }
              onClick={() => setCategory(entry)}
            >
              {t(`dialog.prefs.category.${entry}`)}
            </button>
          ))}
        </div>

        <div className={styles.form}>
          {category === 'general' ? (
            <>
              <label className={styles.row}>
                <span className={styles.rowLabel}>
                  {t('dialog.prefs.theme')}
                </span>
                <select
                  className={styles.text}
                  value={prefs.theme}
                  onChange={(event) =>
                    void update({ theme: event.target.value })
                  }
                >
                  {THEMES.map((theme) => (
                    <option key={theme} value={theme}>
                      {t(`theme.${theme}`)}
                    </option>
                  ))}
                </select>
              </label>
              <label className={styles.row}>
                <span className={styles.rowLabel}>
                  {t('dialog.prefs.language')}
                </span>
                <select
                  className={styles.text}
                  value={prefs.language}
                  onChange={(event) =>
                    void update({ language: event.target.value })
                  }
                >
                  {SUPPORTED_LANGUAGES.map((language) => (
                    <option key={language} value={language}>
                      {t(`language.${language}`)}
                    </option>
                  ))}
                </select>
              </label>
              <div className={`${styles.row} ${styles.rowTop}`}>
                <span className={styles.rowLabel}>
                  {t('dialog.prefs.autosave')}
                </span>
                <div className={styles.rowStack}>
                  <input
                    type="number"
                    min={0}
                    max={3600}
                    aria-label={t('dialog.prefs.autosaveSeconds')}
                    className={styles.number}
                    value={prefs.autosave_seconds}
                    onChange={(event) =>
                      void update({
                        autosave_seconds: Math.max(
                          0,
                          Math.round(Number(event.target.value)),
                        ),
                      })
                    }
                  />
                  <span className={styles.hint}>
                    {t('dialog.prefs.autosaveHint')}
                  </span>
                </div>
              </div>
            </>
          ) : null}

          {category === 'editor' ? (
            <>
              <div className={`${styles.row} ${styles.rowTop}`}>
                <span className={styles.rowLabel}>
                  {t('dialog.prefs.nudge')}
                </span>
                <div className={styles.rowStack}>
                  <div className={styles.nudgeRow}>
                    {NUDGE_STEPS.map((step, index) => (
                      <label key={step} className={styles.nudgeField}>
                        <span className={styles.nudgeLabel}>
                          {t(`dialog.prefs.nudgeStep.${step}`)}
                        </span>
                        <input
                          type="number"
                          min={0}
                          className={styles.number}
                          value={prefs.nudge[index]}
                          onChange={(event) =>
                            setNudge(index, Number(event.target.value))
                          }
                        />
                      </label>
                    ))}
                  </div>
                  <span className={styles.hint}>
                    {t('dialog.prefs.nudgeHint')}
                  </span>
                </div>
              </div>
              <div className={`${styles.row} ${styles.rowTop}`}>
                <span className={styles.rowLabel}>
                  {t('dialog.prefs.gridSize')}
                </span>
                <div className={styles.rowStack}>
                  <input
                    type="number"
                    min={0}
                    step={1}
                    aria-label={t('dialog.prefs.gridSize')}
                    className={styles.number}
                    value={prefs.grid_size}
                    onChange={(event) =>
                      void update({
                        grid_size: Math.max(0, Number(event.target.value)),
                      })
                    }
                  />
                  <span className={styles.hint}>
                    {t('dialog.prefs.gridHint')}
                  </span>
                </div>
              </div>
            </>
          ) : null}

          {category === 'snapping' ? (
            <div className={styles.section}>
              <h3 className={styles.sectionTitle}>
                {t('dialog.prefs.snapTo')}
              </h3>
              {SNAP_FIELDS.map((field) => (
                <div key={field} className={styles.checkRow}>
                  <input
                    id={`snap-${field}`}
                    type="checkbox"
                    checked={prefs.snap[field]}
                    onChange={(event) =>
                      void update({
                        snap: { ...prefs.snap, [field]: event.target.checked },
                      })
                    }
                  />
                  <label
                    htmlFor={`snap-${field}`}
                    className={styles.checkLabel}
                  >
                    {t(`dialog.prefs.snap.${field}`)}
                  </label>
                </div>
              ))}
            </div>
          ) : null}
        </div>
      </div>
    </Dialog>
  )
}
