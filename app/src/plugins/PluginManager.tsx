import { useTranslation } from 'react-i18next'

import { FilterControls } from './FilterControls'
import { useFilterPreview } from './filterSession'
import { usePluginStore } from './pluginStore'
import { bundledPlugins } from './registry'
import styles from './plugins.module.css'

/** Explicit local installation and activation of the curated bundled plugin index. */
export function PluginManager() {
  const { t } = useTranslation()
  const installed = usePluginStore((state) => state.installed)
  const applying = useFilterPreview((state) => state.phase === 'applying')
  return (
    <section className={styles.manager} aria-label={t('plugins.title')}>
      <h3>{t('plugins.title')}</h3>
      <p className={styles.help}>{t('plugins.localOnly')}</p>
      <ul className={styles.list}>
        {bundledPlugins.map((plugin) => {
          const { id, nameKey, descriptionKey, kind, version } = plugin.manifest
          const entry = installed[id]
          const state = usePluginStore.getState()
          return (
            <li className={styles.plugin} key={id}>
              <div className={styles.heading}>
                <strong>{t(nameKey)}</strong>
                <span className={styles.metadata}>
                  {t(`plugins.${kind}`)} · {version}
                </span>
              </div>
              <p>{t(descriptionKey)}</p>
              <div className={styles.actions}>
                <span>
                  {t(
                    entry
                      ? entry.enabled
                        ? 'plugins.enabled'
                        : 'plugins.disabled'
                      : 'plugins.notInstalled',
                  )}
                </span>
                {entry ? (
                  <>
                    <button
                      type="button"
                      disabled={applying}
                      onClick={() =>
                        entry.enabled ? state.disable(id) : state.enable(id)
                      }
                      aria-label={t(
                        entry.enabled
                          ? 'plugins.disableNamed'
                          : 'plugins.enableNamed',
                        { name: t(nameKey) },
                      )}
                    >
                      {t(entry.enabled ? 'plugins.disable' : 'plugins.enable')}
                    </button>
                    <button
                      type="button"
                      disabled={applying}
                      onClick={() => state.remove(id)}
                      aria-label={t('plugins.removeNamed', {
                        name: t(nameKey),
                      })}
                    >
                      {t('plugins.remove')}
                    </button>
                  </>
                ) : (
                  <button
                    type="button"
                    disabled={applying}
                    onClick={() => state.install(id)}
                    aria-label={t('plugins.installNamed', { name: t(nameKey) })}
                  >
                    {t('plugins.install')}
                  </button>
                )}
              </div>
              {kind === 'filter' && entry?.enabled && (
                <FilterControls pluginId={id} />
              )}
            </li>
          )
        })}
      </ul>
    </section>
  )
}
