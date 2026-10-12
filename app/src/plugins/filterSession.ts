import { create } from 'zustand'

import { call, type StyleBatchParams, type StyleBatchResult } from '../api'
import { activeEditor } from '../editor/activeEditor'
import { onDocumentForgotten } from '../stores/documentStore'
import type { FilterPlan, FilterTarget, PluginOverlay } from './contracts'
import { usePluginStore } from './pluginStore'
import { bundledPlugin } from './registry'

/** A reviewed, non-mutating result and the exact request it was computed from. */
export interface FilterPreview {
  pluginId: string
  target: FilterTarget
  generation: number
  plan: FilterPlan
  overlays: readonly PluginOverlay[]
  result: StyleBatchResult
}

/** Dependencies let the lifecycle be tested independently of transport and canvas. */
export interface FilterGateway {
  execute: (request: StyleBatchParams) => Promise<StyleBatchResult>
  isEnabled: (id: string) => boolean
  settle: () => Promise<void>
}

/** UI and canvas share one transient preview, never persisted with plugin preferences. */
export interface FilterState {
  phase: 'idle' | 'loading' | 'ready' | 'applying' | 'error'
  preview: FilterPreview | null
  pending: { pluginId: string; target: FilterTarget } | null
  error: { key: string; detail?: string } | null
  prepare: (
    id: string,
    target: FilterTarget,
    parameters: Readonly<Record<string, number>>,
    label: string,
  ) => Promise<void>
  apply: () => Promise<boolean>
  cancel: () => void
}

function failure(error: unknown): FilterState['error'] {
  const detail = error instanceof Error ? error.message : String(error)
  if (detail.startsWith('plugins.')) return { key: detail }
  if (/preview is stale/i.test(detail)) return { key: 'plugins.stale' }
  return { key: 'plugins.failed', detail }
}

/** Creates a cancellable filter lifecycle. Apply uses the backend's atomic generation guard. */
export function createFilterSession(gateway: FilterGateway) {
  let epoch = 0
  return create<FilterState>((set, get) => ({
    phase: 'idle',
    preview: null,
    pending: null,
    error: null,
    cancel: () => {
      epoch += 1
      set({ phase: 'idle', preview: null, pending: null, error: null })
    },
    prepare: async (id, target, parameters, label) => {
      if (get().phase === 'applying') return
      const requestEpoch = ++epoch
      set({
        phase: 'loading',
        preview: null,
        pending: { pluginId: id, target: { ...target } },
        error: null,
      })
      try {
        const plugin = bundledPlugin(id)
        if (!plugin || !('plan' in plugin) || !gateway.isEnabled(id))
          throw new Error('plugins.notEnabled')
        const capturedTarget = { ...target }
        const plan = plugin.plan(capturedTarget, { ...parameters }, label)
        // Finalise an existing gesture before capturing its revision and outline.
        await gateway.settle()
        if (epoch !== requestEpoch) return
        if (!gateway.isEnabled(id)) {
          get().cancel()
          return
        }
        const result = await gateway.execute({
          ...plan,
          dry_run: true,
          expected_generation: null,
        })
        if (epoch !== requestEpoch) return
        if (!gateway.isEnabled(id)) {
          get().cancel()
          return
        }
        set({
          phase: 'ready',
          pending: null,
          preview: {
            pluginId: id,
            target: capturedTarget,
            generation: result.generation,
            plan,
            overlays: result.examples.map(({ after }) => ({
              kind: 'path',
              path: after,
            })),
            result,
          },
          error: null,
        })
      } catch (error) {
        if (epoch === requestEpoch)
          set({
            phase: 'error',
            preview: null,
            pending: null,
            error: failure(error),
          })
      }
    },
    apply: async () => {
      const preview = get().preview
      if (!preview || get().phase !== 'ready') return false
      const requestEpoch = ++epoch
      set({ phase: 'applying', error: null })
      try {
        if (!gateway.isEnabled(preview.pluginId))
          throw new Error('plugins.notEnabled')
        await gateway.settle()
        if (epoch !== requestEpoch) return false
        if (!gateway.isEnabled(preview.pluginId))
          throw new Error('plugins.notEnabled')
        await gateway.execute({
          ...preview.plan,
          dry_run: false,
          expected_generation: preview.generation,
        })
        if (epoch === requestEpoch)
          set({ phase: 'idle', preview: null, pending: null, error: null })
        return true
      } catch (error) {
        if (epoch === requestEpoch)
          set({
            phase: 'error',
            preview: null,
            pending: null,
            error: failure(error),
          })
        return false
      }
    },
  }))
}

/** Session used by the manager and glyph canvas. */
export const useFilterPreview = createFilterSession({
  execute: (params) => call('style.batch', params),
  isEnabled: (id) => usePluginStore.getState().installed[id]?.enabled === true,
  settle: async () => {
    await activeEditor()?.settle()
  },
})

// Disabled or removed plugins release their drawings immediately.
usePluginStore.subscribe((state) => {
  const session = useFilterPreview.getState()
  const pluginId = session.preview?.pluginId ?? session.pending?.pluginId
  if (pluginId && !state.installed[pluginId]?.enabled)
    useFilterPreview.getState().cancel()
})
onDocumentForgotten((doc) => {
  const session = useFilterPreview.getState()
  if ((session.preview?.target.doc ?? session.pending?.target.doc) === doc)
    useFilterPreview.getState().cancel()
})
