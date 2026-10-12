import { useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { PanelLeft, PanelRight } from 'lucide-react'

import { IconButton } from '../components/IconButton'
import { documentLabel, useSessionStore } from '../stores/sessionStore'
import type { DocId } from '../api'
import {
  reportError,
  selectActiveDoc,
  useUiStore,
  type Tab,
} from '../stores/uiStore'
import { glyphName } from '../views/font/glyphNames'

import styles from './TabBar.module.css'
import { isMacPlatform } from './shortcuts'
import { closeTab, selectTab } from './tabActions'
import { detachTab } from './windowActions'

/**
 * The `Mod+Alt+<n>` binding that selects this tab, in `aria-keyshortcuts`
 * spelling, or `undefined` when no binding reaches it.
 *
 * The numbered commands resolve against the *active* document, so only its tabs
 * announce a binding: a tab of another document would be advertising a shortcut
 * that selects a different tab. Position is counted within the document, with
 * the pinned Overview first, which is what `workspaceTabCommands` selects.
 */
function ordinalShortcut(
  tabs: readonly Tab[],
  tab: Tab,
  activeDoc: DocId | null,
): string | undefined {
  if (tab.doc !== activeDoc) return undefined
  const own = tabs.filter((candidate) => candidate.doc === tab.doc)
  const position =
    tab.kind === 'font'
      ? 0
      : own.filter((candidate) => candidate.kind !== 'font').indexOf(tab) + 1
  if (position < 0 || position > 8) return undefined
  return `${isMacPlatform() ? 'Meta' : 'Control'}+Alt+${position + 1}`
}

/** The tab strip: one Font tab per document plus Glyph and Font Info tabs. */
export function TabBar() {
  const { t } = useTranslation()
  const tabs = useUiStore((state) => state.tabs)
  const activeTab = useUiStore((state) => state.activeTab)
  const sidebarVisible = useUiStore((state) =>
    state.compactPanels
      ? state.compactPanel === 'sidebar'
      : state.panels.sidebar,
  )
  const inspectorVisible = useUiStore((state) =>
    state.compactPanels
      ? state.compactPanel === 'inspector'
      : state.panels.inspector,
  )
  const togglePanel = useUiStore((state) => state.togglePanel)
  const docs = useSessionStore((state) => state.docs)
  const activeDoc = useUiStore(selectActiveDoc)
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
        const pinned = tab.kind === 'font'
        const keyShortcut = ordinalShortcut(tabs, tab, activeDoc)
        return (
          <div
            key={tab.id}
            role="none"
            draggable={tab.kind !== 'font'}
            onDragStart={(event) =>
              event.dataTransfer.setData('text/plain', tab.id)
            }
            onDragEnd={(event) => {
              const bounds = stripRef.current?.getBoundingClientRect()
              if (
                bounds &&
                (event.clientY < bounds.top - 24 ||
                  event.clientY > bounds.bottom + 24)
              )
                void detachTab(tab.id).catch(reportError)
            }}
            className={
              selected ? `${styles.tab} ${styles.tabActive}` : styles.tab
            }
          >
            <button
              type="button"
              role="tab"
              data-tab={tab.id}
              data-pinned={pinned ? 'true' : undefined}
              aria-selected={selected}
              aria-keyshortcuts={keyShortcut}
              // The icon is decorative and a `title` is ignored on a button that
              // has text, so the pinned state is exposed as a description. It
              // stays out of the accessible *name*, which is the tab's title.
              aria-description={pinned ? t('shellWindow.pinned') : undefined}
              tabIndex={selected ? 0 : -1}
              className={styles.tabLabel}
              title={pinned ? `${title} · ${t('shellWindow.pinned')}` : title}
              onClick={() => void selectTab(tab.id)}
              onKeyDown={(event) => onKeyDown(event, index)}
            >
              {pinned ? (
                <span className={styles.tabPin} aria-hidden="true">
                  ⊙
                </span>
              ) : null}
              {pinned && !tab.label
                ? `${t('shellWindow.overview')} · ${title}`
                : title}
            </button>
            {tab.kind === 'font' && summary?.dirty ? (
              <span className={styles.tabDirty} aria-hidden="true" />
            ) : null}
            {tab.kind !== 'font' ? (
              <>
                <IconButton
                  label={t('shellWindow.detachTab', { title })}
                  onClick={() => void detachTab(tab.id).catch(reportError)}
                >
                  ↗
                </IconButton>
                <IconButton
                  label={t('tab.close', { title })}
                  onClick={() => void closeTab(tab.id)}
                >
                  ✕
                </IconButton>
              </>
            ) : null}
          </div>
        )
      })}
      <div className={styles.tabSpacer} role="none" />
      <div className={styles.tabActions} role="none">
        <IconButton
          label={t('a11y.toggleSidebar')}
          aria-pressed={sidebarVisible}
          onClick={() => togglePanel('sidebar')}
        >
          <PanelLeft size={16} strokeWidth={1.5} aria-hidden="true" />
        </IconButton>
        <IconButton
          label={t('a11y.toggleInspector')}
          aria-pressed={inspectorVisible}
          onClick={() => togglePanel('inspector')}
        >
          <PanelRight size={16} strokeWidth={1.5} aria-hidden="true" />
        </IconButton>
      </div>
    </div>
  )
}
