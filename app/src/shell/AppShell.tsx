import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { readWindowTarget } from '../transport/windows'
import { MacroConsole } from '../automation/MacroConsole'
import { installDraftFlushOnWindowBlur } from '../editor/flushOnWindowBlur'
import { DetachedPanel } from './DetachedPanel'
import { DetachedWindowBar } from './DetachedWindowBar'
import { registerWorkspaceTabCommands } from './workspaceTabCommands'
import { installWindowLifecycle } from './windowLifecycle'
import { installWorkspacePersistence } from './workspaceLifecycle'
import { installWorkspaceProjectSync } from '../stores/workspaceProject'
import { WorkspaceDialogHost } from './WorkspaceDialogHost'
import { ShortcutSettingsDialog } from './ShortcutSettingsDialog'
import { ContextHelpDialog } from './ContextHelpDialog'

import { SplitPane } from '../components/SplitPane'
import { ToastStack } from '../components/ToastStack'
import { DialogHost } from '../dialogs/DialogHost'
import { checkForRecovery } from '../dialogs/recovery'
import { i18n } from '../i18n'
import { usePrefsStore } from '../stores/prefsStore'
import { useProgressStore } from '../stores/progressStore'
import { selectSummary, useSessionStore } from '../stores/sessionStore'
import {
  PANEL_BOUNDS,
  applyTheme,
  reportError,
  selectActiveDoc,
  selectActiveTab,
  useUiStore,
} from '../stores/uiStore'

import styles from './AppShell.module.css'
import { registerBuiltinCommands } from './builtinCommands'
import { CloseDocumentDialog } from './CloseDocumentDialog'
import { CommandPalette } from './CommandPalette'
import { CompactPanel, useCompactPanels } from './CompactPanel'
import { setCommandErrorReporter } from './commands'
import { EmptyState } from './EmptyState'
import { Inspector } from './Inspector'
import { MenuBar } from './MenuBar'
import { PathPromptDialog } from './PathPromptDialog'
import { ProgressLine } from './ProgressLine'
import { RecoveredBanner } from './RecoveredBanner'
import { Sidebar } from './Sidebar'
import { installShortcuts } from './shortcuts'
import { StatusBar } from './StatusBar'
import { TabBar } from './TabBar'
import { TabContent } from './TabContent'

/** The application frame: menu bar, tabs, panels, content and status bar. */
export function AppShell() {
  const { t } = useTranslation()
  const [windowTarget] = useState(() => readWindowTarget())
  const detached =
    windowTarget?.kind === 'tab' || windowTarget?.kind === 'panel'
  const theme = useUiStore((state) => state.theme)
  const language = useUiStore((state) => state.language)
  const panels = useUiStore((state) => state.panels)
  const compactPanels = useUiStore((state) => state.compactPanels)
  useCompactPanels()
  const sizes = useUiStore((state) => state.sizes)
  const setPanelSize = useUiStore((state) => state.setPanelSize)
  const activeTab = useUiStore(selectActiveTab)
  const activeDoc = useUiStore(selectActiveDoc)
  const summary = useSessionStore(selectSummary(activeDoc))

  useEffect(() => {
    registerBuiltinCommands()
    registerWorkspaceTabCommands()
    setCommandErrorReporter(reportError)
    const stopWindows = installWindowLifecycle()
    const stopWorkspace = installWorkspacePersistence()
    const stopWorkspaceProject = installWorkspaceProjectSync()
    const stopSession = useSessionStore.getState().start()
    const stopProgress = useProgressStore.getState().start()
    const stopShortcuts = installShortcuts()
    // A detached view's drafts must not stay in that window alone: leaving it
    // is where they reach the kernel, before another window saves or closes.
    const stopDraftFlush = installDraftFlushOnWindowBlur(reportError)
    // The kernel owns the preferences, so they are read before the first paint
    // of anything that depends on them; a crash-recovery listing follows.
    void usePrefsStore.getState().load()
    if (readWindowTarget() === null) void checkForRecovery()
    return () => {
      stopWorkspaceProject()
      stopWorkspace()
      stopWindows()
      stopSession()
      stopProgress()
      stopShortcuts()
      stopDraftFlush()
    }
  }, [])

  useEffect(() => {
    applyTheme(theme)
    if (theme !== 'system') return
    const query = globalThis.matchMedia?.('(prefers-color-scheme: dark)')
    if (!query) return
    const onChange = () => applyTheme('system')
    query.addEventListener('change', onChange)
    return () => query.removeEventListener('change', onChange)
  }, [theme])

  useEffect(() => {
    document.documentElement.lang = language
    if (i18n.language !== language) void i18n.changeLanguage(language)
  }, [language])

  return (
    <div className={styles.shell}>
      <MenuBar />
      {detached ? <DetachedWindowBar target={windowTarget} /> : <TabBar />}
      <ProgressLine />
      <div className={styles.workspace}>
        {windowTarget?.kind !== 'panel' &&
        !compactPanels &&
        summary &&
        activeTab?.kind !== 'bitmap' &&
        panels.sidebar ? (
          <SplitPane
            side="left"
            size={sizes.sidebar}
            min={PANEL_BOUNDS.sidebar.min}
            max={PANEL_BOUNDS.sidebar.max}
            label={t('a11y.sidebarResize')}
            onResize={(size) => setPanelSize('sidebar', size)}
          >
            <Sidebar />
          </SplitPane>
        ) : null}
        <main className={styles.content}>
          {summary ? <RecoveredBanner summary={summary} /> : null}
          {summary && activeTab ? (
            windowTarget?.kind === 'panel' ? (
              <DetachedPanel
                target={windowTarget}
                summary={summary}
                tab={activeTab}
              />
            ) : (
              <TabContent summary={summary} tab={activeTab} />
            )
          ) : (
            <EmptyState />
          )}
        </main>
        {windowTarget?.kind !== 'panel' &&
        !compactPanels &&
        summary &&
        activeTab?.kind !== 'bitmap' &&
        panels.inspector ? (
          <SplitPane
            side="right"
            size={sizes.inspector}
            min={PANEL_BOUNDS.inspector.min}
            max={PANEL_BOUNDS.inspector.max}
            label={t('a11y.inspectorResize')}
            onResize={(size) => setPanelSize('inspector', size)}
          >
            <Inspector />
          </SplitPane>
        ) : null}
      </div>
      <StatusBar />
      {compactPanels &&
      windowTarget?.kind !== 'panel' &&
      summary &&
      activeTab?.kind !== 'bitmap' ? (
        <CompactPanel />
      ) : null}
      <CommandPalette />
      <PathPromptDialog />
      <CloseDocumentDialog />
      <DialogHost />
      <WorkspaceDialogHost />
      <ShortcutSettingsDialog />
      <ContextHelpDialog />
      <MacroConsole />
      <ToastStack />
    </div>
  )
}
