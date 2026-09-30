import type { DocId, GlyphId } from '../api'

import {
  PANEL_BOUNDS,
  tabId,
  type Tab,
  type TabKind,
  type TextTabState,
  type UiState,
} from './uiStore'

/** Saved glyph identity is a name, because numeric handles change on reopening. */
export interface WorkspaceTab {
  kind: TabKind
  glyphName?: string
  label?: string
  textState?: TextTabState
}

/** Document-local tabs and the shell arrangement surrounding them. */
export interface WorkspaceSnapshot {
  tabs: WorkspaceTab[]
  activeTab: number | null
  panels: UiState['panels']
  sizes: UiState['sizes']
  sections: UiState['sections']
  textPreviewVisible: boolean
}

/** The shell fields exchanged by the workspace adapters. */
export type WorkspaceUi = Pick<
  UiState,
  'tabs' | 'activeTab' | 'panels' | 'sizes' | 'sections' | 'textPreviewVisible'
>

const TAB_KINDS: readonly string[] = [
  'font',
  'glyph',
  'fontinfo',
  'features',
  'bitmap',
]
const MAX_TABS = 128

/** Narrows a JSON object without accepting arrays or null. */
export function workspaceRecord(
  value: unknown,
): Record<string, unknown> | null {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null
}

function boundedNumber(
  value: unknown,
  min: number,
  max: number,
  fallback: number,
): number {
  return typeof value === 'number' && Number.isFinite(value)
    ? Math.min(max, Math.max(min, Math.round(value)))
    : fallback
}

function readText(value: unknown): TextTabState | undefined {
  const state = workspaceRecord(value)
  if (
    !state ||
    typeof state.text !== 'string' ||
    state.text.length > 100_000 ||
    !Array.isArray(state.features) ||
    state.features.length > 256 ||
    !state.features.every(
      (feature): feature is [string, boolean] =>
        Array.isArray(feature) &&
        feature.length === 2 &&
        typeof feature[0] === 'string' &&
        /^[\x20-\x7e]{4}$/.test(feature[0]) &&
        typeof feature[1] === 'boolean',
    )
  )
    return undefined
  return {
    text: state.text,
    activeIndex:
      typeof state.activeIndex === 'number' &&
      Number.isSafeInteger(state.activeIndex) &&
      state.activeIndex >= 0
        ? state.activeIndex
        : null,
    mode: state.mode === 'edit' ? 'edit' : 'text',
    master:
      typeof state.master === 'string' && state.master.length <= 1024
        ? state.master
        : null,
    size: boundedNumber(state.size, 1, 4096, 72),
    features: state.features.map(([tag, enabled]) => [tag, enabled]),
    direction:
      state.direction === 'rtl' || state.direction === 'ttb'
        ? state.direction
        : 'ltr',
    previewTheme: state.previewTheme === 'dark' ? 'dark' : 'light',
    previewCollapsed: state.previewCollapsed === true,
    ...(state.spacingSide === 'lsb' ||
    state.spacingSide === 'rsb' ||
    state.spacingSide === 'both'
      ? { spacingSide: state.spacingSide }
      : {}),
    ...(state.kerningMode === 'off' ||
    state.kerningMode === 'on' ||
    state.kerningMode === 'locked'
      ? { kerningMode: state.kerningMode }
      : {}),
    ...(typeof state.script === 'string' && /^[A-Za-z]{4}$/.test(state.script)
      ? { script: state.script }
      : {}),
    ...(typeof state.language === 'string' &&
    /^[A-Za-z]{2,8}(?:-[A-Za-z0-9]{1,8})*$/.test(state.language) &&
    state.language.length <= 63
      ? { language: state.language }
      : {}),
    ...(typeof state.previewHeight === 'number'
      ? { previewHeight: boundedNumber(state.previewHeight, 120, 600, 160) }
      : {}),
  }
}

/** Validates persisted input, discarding invalid tabs and bounding geometry. */
export function readWorkspaceSnapshot(
  value: unknown,
): WorkspaceSnapshot | null {
  const data = workspaceRecord(value)
  const panels = workspaceRecord(data?.panels)
  const sizes = workspaceRecord(data?.sizes)
  if (
    !data ||
    !Array.isArray(data.tabs) ||
    data.tabs.length > MAX_TABS ||
    !panels ||
    !sizes ||
    typeof panels.sidebar !== 'boolean' ||
    typeof panels.inspector !== 'boolean' ||
    typeof data.textPreviewVisible !== 'boolean'
  )
    return null
  const tabs: WorkspaceTab[] = []
  let activeTab: number | null = null
  data.tabs.forEach((raw, index) => {
    const tab = workspaceRecord(raw)
    if (!tab || typeof tab.kind !== 'string' || !TAB_KINDS.includes(tab.kind))
      return
    const textState = readText(tab.textState)
    const glyphName =
      typeof tab.glyphName === 'string' &&
      tab.glyphName.length > 0 &&
      tab.glyphName.length <= 1024
        ? tab.glyphName
        : undefined
    if (tab.kind === 'glyph' && !glyphName && !textState) return
    if (index === data.activeTab) activeTab = tabs.length
    tabs.push({
      kind: tab.kind as TabKind,
      ...(glyphName ? { glyphName } : {}),
      ...(typeof tab.label === 'string' && tab.label.length <= 100_000
        ? { label: tab.label }
        : {}),
      ...(textState ? { textState } : {}),
    })
  })
  const sections = Object.fromEntries(
    Object.entries(workspaceRecord(data.sections) ?? {})
      .filter(([key, value]) => key.length <= 256 && typeof value === 'boolean')
      .slice(0, 256),
  ) as Record<string, boolean>
  return {
    tabs,
    activeTab: activeTab ?? (tabs.length ? 0 : null),
    panels: { sidebar: panels.sidebar, inspector: panels.inspector },
    sizes: {
      sidebar: boundedNumber(
        sizes.sidebar,
        PANEL_BOUNDS.sidebar.min,
        PANEL_BOUNDS.sidebar.max,
        240,
      ),
      inspector: boundedNumber(
        sizes.inspector,
        PANEL_BOUNDS.inspector.min,
        PANEL_BOUNDS.inspector.max,
        300,
      ),
    },
    sections,
    textPreviewVisible: data.textPreviewVisible,
  }
}

/** Copies a trusted snapshot so callers cannot mutate the saved record. */
export function copyWorkspace(snapshot: WorkspaceSnapshot): WorkspaceSnapshot {
  return {
    ...snapshot,
    tabs: snapshot.tabs.map((tab) => ({
      ...tab,
      ...(tab.textState
        ? {
            textState: {
              ...tab.textState,
              features: tab.textState.features.map(([tag, enabled]) => [
                tag,
                enabled,
              ]),
            },
          }
        : {}),
    })),
    panels: { ...snapshot.panels },
    sizes: { ...snapshot.sizes },
    sections: { ...snapshot.sections },
  }
}

/** Captures only one document, replacing ephemeral glyph handles with names. */
export function captureWorkspace(
  doc: DocId,
  ui: WorkspaceUi,
  glyphNames: ReadonlyMap<GlyphId, string> = new Map(),
): WorkspaceSnapshot {
  const tabs: WorkspaceTab[] = []
  let activeTab: number | null = null
  for (const tab of ui.tabs.filter((entry) => entry.doc === doc)) {
    const glyphName =
      tab.gid === undefined ? undefined : glyphNames.get(tab.gid)
    if (tab.kind === 'glyph' && !glyphName && !tab.textState) continue
    if (tab.id === ui.activeTab) activeTab = tabs.length
    const textState = tab.textState ? { ...tab.textState } : undefined
    if (textState) {
      delete textState.runCount
      delete textState.shaping
    }
    tabs.push({
      kind: tab.kind,
      ...(glyphName ? { glyphName } : {}),
      ...(tab.label === undefined ? {} : { label: tab.label }),
      ...(textState ? { textState } : {}),
    })
  }
  return copyWorkspace({
    tabs,
    activeTab: activeTab ?? (tabs.length ? 0 : null),
    panels: ui.panels,
    sizes: ui.sizes,
    sections: ui.sections,
    textPreviewVisible: ui.textPreviewVisible,
  })
}

/** Restores tabs into a reopened document and skips glyphs no longer present. */
export function restoreWorkspace(
  snapshot: WorkspaceSnapshot,
  doc: DocId,
  glyphIds: ReadonlyMap<string, GlyphId> = new Map(),
): WorkspaceUi {
  const saved = copyWorkspace(snapshot)
  const tabs: Tab[] = []
  let activeTab: string | null = null
  saved.tabs.forEach((tab, index) => {
    const gid = tab.glyphName ? glyphIds.get(tab.glyphName) : undefined
    if (tab.kind === 'glyph' && gid === undefined && !tab.textState) return
    const base = tabId(tab.kind, doc, gid)
    let id = base
    for (let suffix = 2; tabs.some((entry) => entry.id === id); suffix += 1)
      id = `${base}:${suffix}`
    const textState = tab.textState
      ? {
          ...tab.textState,
          activeIndex: null,
          mode: 'text' as const,
          shaping: false,
        }
      : undefined
    if (textState) delete textState.runCount
    tabs.push({
      id,
      kind: tab.kind,
      doc,
      ...(gid === undefined ? {} : { gid }),
      ...(tab.label === undefined ? {} : { label: tab.label }),
      ...(textState ? { textState } : {}),
    })
    if (index === saved.activeTab) activeTab = id
  })
  if (!tabs.length) tabs.push({ id: tabId('font', doc), kind: 'font', doc })
  return {
    tabs,
    activeTab: activeTab ?? tabs[0].id,
    panels: saved.panels,
    sizes: saved.sizes,
    sections: saved.sections,
    textPreviewVisible: saved.textPreviewVisible,
  }
}
