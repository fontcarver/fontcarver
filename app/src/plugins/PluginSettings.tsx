import { useTranslation } from 'react-i18next'

import type { PluginIndexEntry } from './contracts'
import { usePluginStore } from './pluginStore'
import { defaultSettings } from './registry'
import styles from './plugins.module.css'

/** Persisted numeric settings declared by one curated index entry. */
export function PluginSettings({ entry }: { entry: PluginIndexEntry }) {
  const { t } = useTranslation()
  const stored = usePluginStore((state) => state.installed[entry.id]?.settings)
  const setSetting = usePluginStore((state) => state.setSetting)
  if (entry.settings.length === 0) return null
  const values = stored ?? defaultSettings(entry.id)
  return (
    <div className={styles.parameters} aria-label={t('plugins.settingsTitle')}>
      {entry.settings.map((setting) => {
        const value = values[setting.key] ?? setting.default
        return setting.kind === 'toggle' ? (
          <label key={setting.key}>
            <input
              type="checkbox"
              checked={value === 1}
              onChange={(event) =>
                setSetting(entry.id, setting.key, event.target.checked ? 1 : 0)
              }
            />{' '}
            {t(setting.labelKey)}
          </label>
        ) : (
          <label key={setting.key}>
            {t(setting.labelKey)}
            <input
              type="number"
              min={setting.min}
              max={setting.max}
              step={setting.step}
              value={value}
              onChange={(event) =>
                setSetting(entry.id, setting.key, Number(event.target.value))
              }
            />
          </label>
        )
      })}
    </div>
  )
}
