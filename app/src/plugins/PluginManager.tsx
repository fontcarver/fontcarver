import { useTranslation } from 'react-i18next'

import { FilterControls } from './FilterControls'
import { useFilterPreview } from './filterSession'
import { PluginSettings } from './PluginSettings'
import { usePluginStore } from './pluginStore'
import { curatedIndex } from './registry'
import styles from './plugins.module.css'

/** Explicit local installation and activation of the curated bundled plugin index. */
export function PluginManager() {
  const { t } = useTranslation()
  const installed = usePluginStore((state) => state.installed)
  const applying = useFilterPreview((state) => state.phase === 'applying')
  const state = usePluginStore.getState()
  return (
    <section className={styles.manager} aria-label={t('plugins.title')}>
      <div className={styles.heading}>
        <h3>{t('plugins.title')}</h3>
        <button
          type="button"
          disabled={applying}
          onClick={() => state.reload()}
          aria-label={t('plugins.reloadIndex')}
        >
          {t('plugins.reloadIndex')}
        </button>
      </div>
      <p className={styles.help}>{t('plugins.localOnly')}</p>
      <ul className={styles.list}>
        {curatedIndex.map((entry) => {
          const { id, nameKey, descriptionKey, kind, version } = entry
          const record = installed[id]
          return (
            <li className={styles.plugin} key={id}>
              <div className={styles.heading}>
                <strong>{t(nameKey)}</strong>
                <span className={styles.metadata}>
                  {t(`plugins.${kind}`)} · {version}
                </span>
              </div>
              <p>{t(descriptionKey)}</p>
              <p className={styles.metadata}>
                {t('plugins.provenance', {
                  author: entry.author,
                  license: entry.license,
                  source: entry.source,
                })}
              </p>
              <div className={styles.actions}>
                <span>
                  {t(
                    record
                      ? record.enabled
                        ? 'plugins.enabled'
                        : 'plugins.disabled'
                      : 'plugins.notInstalled',
                  )}
                </span>
                {record ? (
                  <>
                    <button
                      type="button"
                      disabled={applying}
                      onClick={() =>
                        record.enabled ? state.disable(id) : state.enable(id)
                      }
                      aria-label={t(
                        record.enabled
                          ? 'plugins.disableNamed'
                          : 'plugins.enableNamed',
                        { name: t(nameKey) },
                      )}
                    >
                      {t(record.enabled ? 'plugins.disable' : 'plugins.enable')}
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
              {kind === 'reporter' && record ? (
                <PluginSettings entry={entry} />
              ) : null}
              {kind === 'filter' && record?.enabled && (
                <FilterControls pluginId={id} />
              )}
            </li>
          )
        })}
      </ul>
    </section>
  )
}
