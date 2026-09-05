import { describe, expect, it } from 'vitest'

import { CJK_UNITS_PER_EM, STARTER_RECIPES, metricsFor } from './newFont'

describe('new-font presets', () => {
  it('scales the Latin metrics preset with the units per em', () => {
    expect(metricsFor('latin', 1000)).toMatchObject({
      ascender: 800,
      descender: -200,
      x_height: 500,
      cap_height: 700,
    })
    expect(metricsFor('latin', 2048)).toMatchObject({
      ascender: 1638,
      descender: -410,
      x_height: 1024,
      cap_height: 1434,
    })
  })

  it('uses the CJK preset at 1000 units per em', () => {
    expect(CJK_UNITS_PER_EM).toBe(1000)
    expect(metricsFor('cjk', CJK_UNITS_PER_EM)).toMatchObject({
      ascender: 880,
      descender: -120,
    })
  })

  it('leaves the per-master overrides unset so the compiler derives them', () => {
    const metrics = metricsFor('latin', 1000)

    expect(metrics.hhea_ascender).toBeNull()
    expect(metrics.typo_descender).toBeNull()
    expect(metrics.win_ascent).toBeNull()
  })

  it('describes each starter set as a kernel recipe', () => {
    expect(STARTER_RECIPES.none).toBe('')
    expect(STARTER_RECIPES.basicLatin.startsWith('A-Z a-z 0-9')).toBe(true)
    expect(STARTER_RECIPES.latin1).toBe('U+0020-U+00FF')
    expect(STARTER_RECIPES.gb2312).toBe('@charset:GB2312')
  })
})
