import { useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { IconButton } from '../components/IconButton'
import { documentLabel, useSessionStore } from '../stores/sessionStore'
import { useUiStore, type Tab } from '../stores/uiStore'
import { glyphName } from '../views/font/glyphNames'

import styles from './TabBar.module.css'
import { closeTab, selectTab } from './tabActions'

/** The tab strip: one Font tab per document plus Glyph and Font Info tabs. */
export function TabBar() {
  const { t } = useTranslation()
  const tabs = useUiStore((state) => state.tabs)
  const activeTab = useUiStore((state) => state.activeTab)
  const panels = useUiStore((state) => state.panels)
  const togglePanel = useUiStore((state) => state.togglePanel)
  const docs = useSessionStore((state) => state.docs)
  const stripRef = useRef<HTMLDivElement>(null)

  const titleOf = (tab: Tab): string => {
    if (tab.label) return tab.label
    const summary = docs.find((doc) => doc.doc === tab.doc)
    if (tab.kind === 'font') return documentLabel(summary)
    if (tab.kind === 'fontinfo') return t('tab.fontinfo')
    if (tab.kind === 'bitmap') return t('tab.bitmap')
    if (tab.kind === 'features') return t('tab.features')
    const name = tab.gid === undefined ? undefined : glyphName(tab.doc, tab.gid)
    return name ?? t('view.glyphLabel', { gid: tab.gid })
  }

  const onKeyDown = (event: React.KeyboardEvent, index: number) => {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
    event.preventDefault()
    const delta = event.key === 'ArrowRight' ? 1 : -1
    const next = tabs[(index + delta + tabs.length) % tabs.length]
    if (!next) return
    void selectTab(next.id).then((selected) => {
      if (!selected || useUiStore.getState().activeTab !== next.id) return
      stripRef.current
        ?.querySelector<HTMLButtonElement>(`[data-tab="${next.id}"]`)
        ?.focus()
    })
  }

  return (
    <div
      ref={stripRef}
      className={styles.tabbar}
      role="tablist"
      aria-label={t('a11y.tabs')}
    >
      {tabs.map((tab, index) => {
        const summary = docs.find((doc) => doc.doc === tab.doc)
        const title = titleOf(tab)
        const selected = tab.id === activeTab
        return (
          <div
            key={tab.id}
            role="none"
            className={
              selected ? `${styles.tab} ${styles.tabActive}` : styles.tab
            }
          >
            <button
              type="button"
              role="tab"
              data-tab={tab.id}
              aria-selected={selected}
              tabIndex={selected ? 0 : -1}
              className={styles.tabLabel}
              title={title}
              onClick={() => void selectTab(tab.id)}
              onKeyDown={(event) => onKeyDown(event, index)}
            >
              {title}
            </button>
            {tab.kind === 'font' && summary?.dirty ? (
              <span className={styles.tabDirty} aria-hidden="true" />
            ) : null}
            <IconButton
              label={t('tab.close', { title })}
              onClick={() => void closeTab(tab.id)}
            >
              ✕
            </IconButton>
          </div>
        )
      })}
      <div className={styles.tabSpacer} role="none" />
      <div className={styles.tabActions} role="none">
        <IconButton
          label={t('a11y.toggleSidebar')}
          aria-pressed={panels.sidebar}
          onClick={() => togglePanel('sidebar')}
        >
          ▤
        </IconButton>
        <IconButton
          label={t('a11y.toggleInspector')}
          aria-pressed={panels.inspector}
          onClick={() => togglePanel('inspector')}
        >
          ▥
        </IconButton>
      </div>
    </div>
  )
}
