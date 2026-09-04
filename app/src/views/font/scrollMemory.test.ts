import { beforeEach, describe, expect, it } from 'vitest'

import type { GlyphSort } from '../../api'
import { emptyFilter } from '../../stores/documentStore'

import type { GlyphQuery } from './glyphSelection'
import {
  forgetScroll,
  rememberScroll,
  rememberedScroll,
  resetScrollMemory,
} from './scrollMemory'

const DOC = 3

function query(sort: GlyphSort = 'Order', doc = DOC): GlyphQuery {
  return { doc, master: null, filter: emptyFilter(), sort }
}

beforeEach(() => {
  resetScrollMemory()
})

describe('scroll memory', () => {
  it('reports nothing for a list it has never seen', () => {
    expect(rememberedScroll(query())).toBeNull()
  })

  it('returns the offset last recorded for a list', () => {
    rememberScroll(query(), 1_440)

    expect(rememberedScroll(query())).toBe(1_440)
  })

  it('keeps one offset per filtered, sorted list', () => {
    rememberScroll(query(), 1_440)
    rememberScroll(query('Unicode'), 96)

    expect(rememberedScroll(query())).toBe(1_440)
    expect(rememberedScroll(query('Unicode'))).toBe(96)
  })

  it('forgets every list of a document that closed', () => {
    rememberScroll(query(), 1_440)
    rememberScroll(query('Order', 4), 720)

    forgetScroll(DOC)

    expect(rememberedScroll(query())).toBeNull()
    expect(rememberedScroll(query('Order', 4))).toBe(720)
  })

  it('drops the least recently used list past its cap', () => {
    for (let index = 0; index < 12; index += 1)
      rememberScroll(query('Order', index), index * 10)

    expect(rememberedScroll(query('Order', 0))).toBeNull()
    expect(rememberedScroll(query('Order', 11))).toBe(110)
  })
})
