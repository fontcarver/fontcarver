import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type FontInfoView } from '../api'
import { makeFontInfo, makeFontInfoView } from '../test/fontFixtures'
import { createMockTransport, type MockTransport } from '../test/mockTransport'

import {
  cachedFontInfo,
  forgetFontInfo,
  invalidateFontInfo,
  loadFontInfo,
} from './fontInfoCache'

const DOC = 1

const STALE = makeFontInfoView({
  info: makeFontInfo({ family_name: 'Before Edit' }),
})
const FRESH = makeFontInfoView({
  info: makeFontInfo({ family_name: 'After Edit' }),
})

/** A transport whose `doc.font_info` answers only when the test says so. */
function deferredTransport(): {
  transport: MockTransport
  answer: (index: number, view: FontInfoView) => void
  requests: () => number
} {
  const pending: ((view: FontInfoView) => void)[] = []
  const transport = createMockTransport({
    'doc.font_info': () =>
      new Promise<FontInfoView>((resolve) => pending.push(resolve)),
  })
  return {
    transport,
    answer: (index, view) => pending[index]?.(view),
    requests: () =>
      transport.calls.filter((entry) => entry.method === 'doc.font_info')
        .length,
  }
}

beforeEach(() => {
  forgetFontInfo(DOC)
})

afterEach(() => {
  setTransport(undefined)
  forgetFontInfo(DOC)
})

describe('font info cache', () => {
  it('shares one request between callers', async () => {
    const { transport, answer, requests } = deferredTransport()
    setTransport(transport)

    const first = loadFontInfo(DOC)
    const second = loadFontInfo(DOC)
    answer(0, FRESH)

    expect(await first).toBe(FRESH)
    expect(await second).toBe(FRESH)
    expect(requests()).toBe(1)
  })

  it('refetches instead of joining a request the invalidation overtook', () => {
    const { transport, requests } = deferredTransport()
    setTransport(transport)

    const stale = loadFontInfo(DOC)
    invalidateFontInfo(DOC)
    const fresh = loadFontInfo(DOC)

    expect(requests()).toBe(2)
    expect(stale).not.toBe(fresh)
  })

  it('never caches a response computed before the invalidation', async () => {
    const { transport, answer, requests } = deferredTransport()
    setTransport(transport)

    const stale = loadFontInfo(DOC)
    invalidateFontInfo(DOC)
    const fresh = loadFontInfo(DOC)

    // The overtaken request answers last, which is the whole hazard.
    answer(0, STALE)
    await stale
    expect(cachedFontInfo(DOC)).toBeUndefined()

    answer(1, FRESH)
    expect(await fresh).toBe(FRESH)
    expect(cachedFontInfo(DOC)).toBe(FRESH)
    expect(requests()).toBe(2)
  })

  it('keeps the newer request reachable after the older one settles', async () => {
    const { transport, answer, requests } = deferredTransport()
    setTransport(transport)

    void loadFontInfo(DOC)
    invalidateFontInfo(DOC)
    const fresh = loadFontInfo(DOC)
    answer(0, STALE)
    await Promise.resolve()

    // The stale request must not have evicted the live one from the in-flight
    // map, or a third caller would start a duplicate request.
    const joined = loadFontInfo(DOC)
    expect(joined).toBe(fresh)
    expect(requests()).toBe(2)
    answer(1, FRESH)
    expect(await joined).toBe(FRESH)
  })
})
