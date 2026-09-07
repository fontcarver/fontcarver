import { useEffect } from 'react'
import { useUiStore } from '../../stores/uiStore'

/** Publishes derived run feedback to the shell without mixing export progress. */
export function useTextFeedback(
  tabId: string,
  runCount: number,
  shaping: boolean,
): void {
  useEffect(() => {
    const ui = useUiStore.getState()
    const tab = ui.tabs.find((tab) => tab.id === tabId)
    if (!tab?.textState) return
    if (
      tab.textState.runCount === runCount &&
      tab.textState.shaping === shaping
    )
      return
    ui.updateTab(tabId, { textState: { ...tab.textState, runCount, shaping } })
  }, [runCount, shaping, tabId])
}
