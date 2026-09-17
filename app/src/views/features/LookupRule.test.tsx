import { fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeAll, describe, expect, it } from 'vitest'

import { setTransport } from '../../api'
import { i18n } from '../../i18n'
import { createMockTransport } from '../../test/mockTransport'

import english from '../../i18n/en.json'
import { LookupRule } from './LookupRule'

beforeAll(() =>
  i18n.addResourceBundle('en', 'translation', english, true, true),
)
afterEach(() => setTransport(undefined))

describe('LookupRule', () => {
  it('pages a large class beyond its initial summary with only eight references per request', async () => {
    const transport = createMockTransport({
      'layout.rule_glyphs': ({ offset }) => ({
        doc: 1,
        generation: 3,
        compile_revision: 2,
        draft_revision: 0,
        fingerprint: 'abc',
        table: 'GSUB',
        lookup: 4,
        rule: 12,
        total: 80,
        items: Array.from({ length: 8 }, (_, index) => ({
          name: `glyph${offset + index}`,
          gid: null,
        })),
        next_offset: offset + 8 < 80 ? offset + 8 : null,
      }),
    })
    setTransport(transport)
    render(
      <LookupRule
        doc={1}
        master={null}
        generation={3}
        table="GSUB"
        lookup={4}
        index={12}
        rule={{ text: 'sub [large class] by b;', glyphs: [], truncated: true }}
      />,
    )
    await screen.findByText('glyph0')
    for (let offset = 8; offset <= 64; offset += 8) {
      fireEvent.click(screen.getByRole('button', { name: 'Next glyphs' }))
      await screen.findByText(`glyph${offset}`)
    }
    expect(screen.queryByText('glyph0')).not.toBeInTheDocument()
    expect(screen.getByText('glyph71')).toBeVisible()
    expect(transport.calls.at(-1)).toEqual({
      method: 'layout.rule_glyphs',
      params: {
        doc: 1,
        table: 'GSUB',
        lookup: 4,
        rule: 12,
        generation: 3,
        offset: 64,
        limit: 8,
      },
    })
  })

  it('keeps source text available when its glyph page fails and supports retry', async () => {
    let failed = true
    setTransport(
      createMockTransport({
        'layout.rule_glyphs': () => {
          if (failed) throw new Error('lost')
          return {
            doc: 1,
            generation: 3,
            compile_revision: 2,
            draft_revision: 0,
            fingerprint: 'abc',
            table: 'GSUB',
            lookup: 0,
            rule: 0,
            total: 1,
            items: [{ name: 'a', gid: null }],
            next_offset: null,
          }
        },
      }),
    )
    render(
      <LookupRule
        doc={1}
        master={null}
        generation={3}
        table="GSUB"
        lookup={0}
        index={0}
        rule={{ text: 'sub a by b;', glyphs: [], truncated: false }}
      />,
    )
    expect(screen.getByText('sub a by b;')).toBeVisible()
    const retry = await screen.findByRole('button', { name: 'Retry glyphs' })
    failed = false
    fireEvent.click(retry)
    expect(await screen.findByText('a')).toBeVisible()
  })
})
