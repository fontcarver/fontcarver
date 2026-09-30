/**
 * Per-tab beam and cut-off display state.
 *
 * These are view settings, not document data: they belong to the tab the user
 * is measuring in, they must survive a re-render, and the canvas has to see
 * them through the run object so a change repaints. A tiny store keyed by tab
 * id gives all three without widening the workspace snapshot, which another
 * task owns.
 */

import { create } from 'zustand'

import type { SpacingViewState } from './spacingMatrix'

interface SpacingViewStore {
  byTab: Record<string, Partial<SpacingViewState>>
  update: (tab: string, changes: Partial<SpacingViewState>) => void
  forget: (tab: string) => void
}

export const useSpacingViewStore = create<SpacingViewStore>((set) => ({
  byTab: {},
  update: (tab, changes) =>
    set((state) => ({
      byTab: { ...state.byTab, [tab]: { ...state.byTab[tab], ...changes } },
    })),
  forget: (tab) =>
    set((state) => {
      if (!(tab in state.byTab)) return state
      const byTab = { ...state.byTab }
      delete byTab[tab]
      return { byTab }
    }),
}))

/** Subscribes to one tab's stored spacing display choices. */
export function useSpacingViewChoices(
  tab: string,
): Partial<SpacingViewState> | undefined {
  return useSpacingViewStore((state) => state.byTab[tab])
}
