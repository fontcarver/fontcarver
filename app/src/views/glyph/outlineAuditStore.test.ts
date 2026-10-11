import { beforeEach, describe, expect, it, vi } from 'vitest'

import { DEFAULT_OUTLINE_AUDIT } from './outlineAuditModel'
import { readStoredAudit, useOutlineAuditStore } from './outlineAuditStore'

const KEY = 'fontcarver.outlineAudit'

describe('outline audit switches persist per viewer', () => {
  beforeEach(() => {
    globalThis.localStorage.clear()
    useOutlineAuditStore.setState({ settings: { ...DEFAULT_OUTLINE_AUDIT } })
  })

  it('writes a disabled check and reads it back on the next session', () => {
    useOutlineAuditStore.getState().setTest('coincident', false)
    expect(useOutlineAuditStore.getState().settings.coincident).toBe(false)
    expect(
      JSON.parse(globalThis.localStorage.getItem(KEY) ?? '{}'),
    ).toMatchObject({ coincident: false, extrema: true })
    expect(readStoredAudit().coincident).toBe(false)
  })

  it('falls back to every check on for missing, broken or foreign values', () => {
    expect(readStoredAudit()).toEqual(DEFAULT_OUTLINE_AUDIT)
    globalThis.localStorage.setItem(KEY, 'not json at all')
    expect(readStoredAudit()).toEqual(DEFAULT_OUTLINE_AUDIT)
    globalThis.localStorage.setItem(KEY, '"a string"')
    expect(readStoredAudit()).toEqual(DEFAULT_OUTLINE_AUDIT)
    globalThis.localStorage.setItem(KEY, '{"open":"yes","extrema":false}')
    expect(readStoredAudit()).toEqual({
      ...DEFAULT_OUTLINE_AUDIT,
      extrema: false,
    })
  })

  it('keeps working when site data is unavailable', () => {
    const getItem = vi
      .spyOn(globalThis.localStorage, 'getItem')
      .mockImplementation(() => {
        throw new Error('site data blocked')
      })
    const setItem = vi
      .spyOn(globalThis.localStorage, 'setItem')
      .mockImplementation(() => {
        throw new Error('site data blocked')
      })
    try {
      expect(readStoredAudit()).toEqual(DEFAULT_OUTLINE_AUDIT)
      useOutlineAuditStore.getState().setTest('open', false)
      expect(useOutlineAuditStore.getState().settings.open).toBe(false)
    } finally {
      getItem.mockRestore()
      setItem.mockRestore()
    }
  })
})
