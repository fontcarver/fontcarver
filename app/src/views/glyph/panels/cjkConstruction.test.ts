import { describe, expect, it } from 'vitest'
import type { Component } from '../../../api'
import { IdsError, idsCommand, layoutIds } from './cjkConstruction'

const layout = (text: string) => layoutIds(text, 1000, 1000, -200)

function expectCode(text: string, code: IdsError['code']) {
  expect(() => layout(text)).toThrow(IdsError)
  try {
    layout(text)
  } catch (error) {
    expect(error).toMatchObject({ code })
  }
}

function balanced(count: number): string {
  if (count === 1) return '木'
  const left = Math.floor(count / 2)
  return `⿰${balanced(left)}${balanced(count - left)}`
}

describe('layoutIds', () => {
  it('keeps single scalars and explicit glyph names as identity leaves', () => {
    expect(layout('𠀀')).toEqual([
      { token: '𠀀', transform: [1, 0, 0, 1, 0, 0] },
    ])
    expect(layout(' [radical.left] ')).toEqual([
      { token: '[radical.left]', transform: [1, 0, 0, 1, 0, 0] },
    ])
    expect(layout('[木.alt 1]')[0].token).toBe('[木.alt 1]')
  })

  it('composes nested horizontal and top-to-bottom cells in font coordinates', () => {
    expect(layout('⿰ 木 ⿱日月')).toEqual([
      { token: '木', transform: [0.5, 0, 0, 1, 0, 0] },
      { token: '日', transform: [0.5, 0, 0, 0.5, 500, 400] },
      { token: '月', transform: [0.5, 0, 0, 0.5, 500, -100] },
    ])
    expect(layoutIds('⿱木月', 600, 1200, -300)).toEqual([
      { token: '木', transform: [1, 0, 0, 0.5, 0, 450] },
      { token: '月', transform: [1, 0, 0, 0.5, 0, -150] },
    ])
  })

  it('lays out ternary operators in input order', () => {
    const horizontal = layout('⿲木日月')
    const vertical = layout('⿳木日月')
    for (let i = 0; i < 3; i += 1) {
      expect(horizontal[i].transform[0]).toBeCloseTo(1 / 3)
      expect(horizontal[i].transform[4]).toBeCloseTo((i * 1000) / 3)
      expect(vertical[i].transform[3]).toBeCloseTo(1 / 3)
      expect(vertical[i].transform[5]).toBeCloseTo(((2 - i) * 1000 - 400) / 3)
    }
  })

  it.each([
    ['⿴', [0.5, 0, 0, 0.5, 250, 150]],
    ['⿵', [0.5, 0, 0, 0.75, 250, -50]],
    ['⿶', [0.5, 0, 0, 0.75, 250, 200]],
    ['⿷', [0.75, 0, 0, 0.5, 250, 150]],
    ['⿸', [0.75, 0, 0, 0.75, 250, -50]],
    ['⿹', [0.75, 0, 0, 0.75, 0, -50]],
    ['⿺', [0.75, 0, 0, 0.75, 250, 200]],
    ['⿻', [1, 0, 0, 1, 0, 0]],
  ])(
    'uses a full outer cell and an editable inner template for %s',
    (operator, inner) => {
      const leaves = layout(`${operator}囗木`)
      expect(leaves[0].transform).toEqual([1, 0, 0, 1, 0, 0])
      expect(leaves[1].transform).toEqual(inner)
    },
  )

  it('composes nested enclosure and overlay templates', () => {
    expect(layout('⿴囗⿻日月')).toEqual([
      { token: '囗', transform: [1, 0, 0, 1, 0, 0] },
      { token: '日', transform: [0.5, 0, 0, 0.5, 250, 150] },
      { token: '月', transform: [0.5, 0, 0, 0.5, 250, 150] },
    ])
  })

  it.each([
    '',
    ' ',
    '⿰木',
    '⿲木日',
    '木日',
    '⿱木日月',
    '[]',
    '[ ]',
    '[木',
    '木]',
    '[[木]]',
    '\ud800',
    '\u0000',
  ])('rejects malformed input %j', (text) => {
    expectCode(text, 'invalid')
  })

  it.each(['⿼', '⿽', '⿾', '⿿', '㇯'])(
    'explicitly rejects unsupported operator %s',
    (operator) => {
      expectCode(`${operator}木日`, 'operator')
      expectCode(`⿰木${operator}日`, 'operator')
    },
  )

  it('bounds total scalars, leaves and operator nesting independently', () => {
    expect(layout(`[${'𠀀'.repeat(510)}]`)).toHaveLength(1)
    expectCode(`[${'𠀀'.repeat(511)}]`, 'limit')
    expect(layout(balanced(64))).toHaveLength(64)
    expectCode(balanced(65), 'limit')
    expect(layout(`${'⿰木'.repeat(16)}木`)).toHaveLength(17)
    expectCode(`${'⿰木'.repeat(17)}木`, 'limit')
  })

  it.each([
    [0, 1000, -200],
    [-1, 1000, -200],
    [Infinity, 1000, -200],
    [1000, 0, -200],
    [1000, NaN, -200],
    [1000, 1000, Infinity],
  ])('rejects unusable metrics %j %j %j', (width, upem, descender) => {
    expect(() => layoutIds('木', width, upem, descender)).toThrow(IdsError)
  })
})

describe('idsCommand', () => {
  it('appends every component atomically with all flags intact', () => {
    const component: Component = {
      base: 'radical.left',
      transform: [0.5, 0.1, -0.2, 1, 40, 20],
      anchor: 'top',
      auto_align: true,
      use_my_metrics: true,
      round_to_grid: true,
      smart: { Weight: 500 },
      point_match: [2, 4],
    }
    const second: Component = { ...component, base: 'radical.right' }
    expect(
      idsCommand({ doc: 1, glyph: 'target', layer: 'regular' }, [
        component,
        second,
      ]),
    ).toEqual({
      Batch: [component, second].map((entry) => ({
        AddComponent: { glyph: 'target', layer: 'regular', component: entry },
      })),
    })
  })
})
