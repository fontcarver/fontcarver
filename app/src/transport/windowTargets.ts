import type { DocId, GlyphId } from '../api/generated'
import type { PanelId, Tab, TabKind } from '../stores/uiStore'

/** A view of a document in the shared kernel session. */
export type WindowTarget =
  | { kind: 'document'; doc: DocId }
  | { kind: 'tab'; doc: DocId; tab: TabKind; gid?: GlyphId; tabId?: string }
  | { kind: 'panel'; doc: DocId; panel: PanelId | 'preview' }

const tabKinds = new Set(['font', 'glyph', 'fontinfo', 'features', 'bitmap'])
const panelKinds = new Set(['sidebar', 'inspector', 'preview'])
const tabIdPattern = /^[a-zA-Z0-9_:.-]{1,128}$/
const isId = (value: unknown): value is number =>
  typeof value === 'number' &&
  Number.isInteger(value) &&
  value >= 0 &&
  value <= 0xffff_ffff

/** Validates untrusted URL, storage and cross-window event payloads. */
export function isWindowTarget(value: unknown): value is WindowTarget {
  if (!value || typeof value !== 'object') return false
  const target = value as Record<string, unknown>
  if (!isId(target.doc)) return false
  if (target.kind === 'document') return true
  if (target.kind === 'panel')
    return typeof target.panel === 'string' && panelKinds.has(target.panel)
  return (
    target.kind === 'tab' &&
    typeof target.tab === 'string' &&
    tabKinds.has(target.tab) &&
    (target.gid === undefined || isId(target.gid)) &&
    (target.tabId === undefined ||
      (typeof target.tabId === 'string' && tabIdPattern.test(target.tabId)))
  )
}

/** Decodes a detached view route, ignoring malformed or unrelated URLs. */
export function readWindowTarget(
  search = window.location.search,
): WindowTarget | null {
  const params = new URLSearchParams(search)
  const docText = params.get('doc')
  if (!docText || !/^\d+$/.test(docText)) return null
  const common = { kind: params.get('fcWindow'), doc: Number(docText) }
  const target =
    common.kind === 'tab'
      ? {
          ...common,
          tab: params.get('tab'),
          ...(params.has('gid')
            ? {
                gid: /^\d+$/.test(params.get('gid') ?? '')
                  ? Number(params.get('gid'))
                  : NaN,
              }
            : {}),
          ...(params.has('tabId') ? { tabId: params.get('tabId') } : {}),
        }
      : common.kind === 'panel'
        ? { ...common, panel: params.get('panel') }
        : common
  return isWindowTarget(target) ? target : null
}

/** Builds a relative application URL containing only the view identity. */
export function windowTargetUrl(target: WindowTarget): string {
  if (!isWindowTarget(target))
    throw new Error('Invalid workspace window target')
  const query = new URLSearchParams({
    fcWindow: target.kind,
    doc: String(target.doc),
  })
  if (target.kind === 'tab') {
    query.set('tab', target.tab)
    if (target.gid !== undefined) query.set('gid', String(target.gid))
    if (target.tabId !== undefined) query.set('tabId', target.tabId)
  } else if (target.kind === 'panel') query.set('panel', target.panel)
  return `?${query}`
}

/** Stable ASCII window name, shared with the native shell implementation. */
export function windowTargetLabel(target: WindowTarget): string {
  if (!isWindowTarget(target))
    throw new Error('Invalid workspace window target')
  let label = `fc-${target.kind}-${target.doc}`
  if (target.kind === 'panel') label += `-${target.panel}`
  if (target.kind === 'tab') {
    label += `-${target.tab}`
    if (target.tabId !== undefined) {
      label += `-id-${Array.from(target.tabId, (char) => char.charCodeAt(0).toString(16).padStart(2, '0')).join('')}`
    } else label += `-${target.gid ?? 'none'}`
  }
  return label
}

function payloadKey(target: WindowTarget): string {
  return `fontcarver:window-tab:${windowTargetLabel(target)}`
}

function matchingTab(value: unknown, target: WindowTarget): value is Tab {
  if (!value || typeof value !== 'object') return false
  const tab = value as Partial<Tab>
  if (
    typeof tab.id !== 'string' ||
    tab.doc !== target.doc ||
    !tab.kind ||
    !tabKinds.has(tab.kind)
  )
    return false
  if (tab.gid !== undefined && !isId(tab.gid)) return false
  if (
    target.kind === 'tab' &&
    (tab.kind !== target.tab ||
      (target.tabId !== undefined
        ? tab.id !== target.tabId
        : tab.gid !== target.gid))
  )
    return false
  const text = tab.textState
  return (
    text === undefined ||
    (typeof text === 'object' &&
      text !== null &&
      typeof text.text === 'string' &&
      (text.mode === 'text' || text.mode === 'edit') &&
      (text.activeIndex === null ||
        (Number.isInteger(text.activeIndex) && text.activeIndex >= 0)) &&
      (text.master === null || typeof text.master === 'string') &&
      Number.isFinite(text.size) &&
      text.size > 0 &&
      ['ltr', 'rtl', 'ttb'].includes(text.direction) &&
      ['light', 'dark'].includes(text.previewTheme) &&
      typeof text.previewCollapsed === 'boolean' &&
      Array.isArray(text.features) &&
      text.features.every(
        (feature) =>
          Array.isArray(feature) &&
          feature.length === 2 &&
          typeof feature[0] === 'string' &&
          typeof feature[1] === 'boolean',
      ))
  )
}

/** Persists a detached tab's latest view state without putting text in URLs. */
export function writeWindowTab(target: WindowTarget, tab: Tab): void {
  if (!matchingTab(tab, target))
    throw new Error('Tab does not match workspace window target')
  localStorage.setItem(payloadKey(target), JSON.stringify(tab))
}

/** Restores a matching tab payload; corrupt or unrelated stored data is ignored. */
export function readWindowTab(target: WindowTarget): Tab | null {
  try {
    const value: unknown = JSON.parse(
      localStorage.getItem(payloadKey(target)) ?? 'null',
    )
    return matchingTab(value, target) ? value : null
  } catch {
    return null
  }
}
