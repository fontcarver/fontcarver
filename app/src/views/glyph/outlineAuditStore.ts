/**
 * Which outline-audit checks are on, remembered between sessions.
 *
 * A designer turns a check off because it is noise *for their font*, not for
 * one glyph, so the switch has to outlive the tab. It is a per-viewer
 * convenience rather than document data, which is why it lives in
 * `localStorage` and not in the `.fcf`: reading it is wrapped so a private
 * window or blocked site data simply falls back to every check on.
 */

import { create } from 'zustand'

import {
  DEFAULT_OUTLINE_AUDIT,
  OUTLINE_AUDIT_TESTS,
  type OutlineAuditSettings,
  type OutlineAuditTest,
} from './outlineAudit'

const STORAGE_KEY = 'fontcarver.outlineAudit'

/** The stored settings, or the defaults when nothing usable is stored. */
export function readStoredAudit(): OutlineAuditSettings {
  try {
    const raw = globalThis.localStorage?.getItem(STORAGE_KEY)
    if (!raw) return DEFAULT_OUTLINE_AUDIT
    const parsed: unknown = JSON.parse(raw)
    if (typeof parsed !== 'object' || parsed === null)
      return DEFAULT_OUTLINE_AUDIT
    const record = parsed as Partial<Record<OutlineAuditTest, unknown>>
    const settings = { ...DEFAULT_OUTLINE_AUDIT }
    for (const test of OUTLINE_AUDIT_TESTS)
      if (typeof record[test] === 'boolean') settings[test] = record[test]
    return settings
  } catch {
    return DEFAULT_OUTLINE_AUDIT
  }
}

function writeStoredAudit(settings: OutlineAuditSettings): void {
  try {
    globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(settings))
  } catch {
    // A viewer with site data blocked keeps the switches for this session only.
  }
}

/** The audit's per-check switches. */
export interface OutlineAuditState {
  settings: OutlineAuditSettings
  setTest: (test: OutlineAuditTest, enabled: boolean) => void
}

export const useOutlineAuditStore = create<OutlineAuditState>((set, get) => ({
  settings: readStoredAudit(),
  setTest: (test, enabled) => {
    if (get().settings[test] === enabled) return
    const settings = { ...get().settings, [test]: enabled }
    writeStoredAudit(settings)
    set({ settings })
  },
}))
