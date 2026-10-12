import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'

import {
  setTransport,
  type LayoutInspection,
  type LayoutRules,
} from '../../api'
import { i18n } from '../../i18n'
import { cellFor } from '../../test/glyphFixtures'
import { createMockTransport } from '../../test/mockTransport'

import english from '../../i18n/en.json'
import { LookupInspector } from './LookupInspector'

vi.mock('./LayoutConversion', () => ({ LayoutConversion: () => null }))

beforeAll(() =>
  i18n.addResourceBundle('en', 'translation', english, true, true),
)
afterEach(() => setTransport(undefined))

function inspection(
  overrides: Partial<LayoutInspection> = {},
): LayoutInspection {
  return {
    doc: 1,
    generation: 3,
    compile_revision: 2,
    draft_revision: 0,
    authority: 'ImportedTables',
    fingerprint: 'abc',
    tables: [{ tag: 'GSUB', byte_length: 40, lookup_count: 1 }],
    blockers: [],
    blockers_truncated: false,
    can_convert: true,
    original_layout_available: false,
    total: 1,
    items: [
      {
        table: 'GSUB',
        index: 0,
        kind: 1,
        flags: 0,
        referenced_lookups: [],
        references_truncated: false,
        rule_count: 1,
      },
    ],
    next_offset: null,
    ...overrides,
  }
}

function rules(overrides: Partial<LayoutRules> = {}): LayoutRules {
  return {
    doc: 1,
    generation: 3,
    compile_revision: 2,
    draft_revision: 0,
    fingerprint: 'abc',
    table: 'GSUB',
    lookup: 0,
    total: 1,
    items: [
      {
        text: 'sub a by a.alt;',
        glyphs: [{ name: 'a.alt', gid: 1 }],
        truncated: false,
      },
    ],
    next_offset: null,
    ...overrides,
  }
}

describe('LookupInspector', () => {
  it('shows original rules and glyph outlines without applying any source', async () => {
    const transport = createMockTransport({
      'layout.inspect': () => inspection(),
      'layout.rules': () => rules(),
      'layout.rule_glyphs': () => ({
        ...rules(),
        rule: 0,
        items: [{ name: 'a.alt', gid: 1 }],
      }),
      'glyphs.cell': () => cellFor(1, { name: 'a.alt' }),
    })
    setTransport(transport)
    render(
      <LookupInspector
        doc={1}
        master="m1"
        generation={3}
        onChanged={vi.fn()}
      />,
    )
    expect(await screen.findByText('sub a by a.alt;')).toBeVisible()
    expect(
      await screen.findByRole('img', { name: 'Outline of a.alt' }),
    ).toBeVisible()
    expect(
      screen.getByText('Original layout rules are read-only.'),
    ).toBeVisible()
    expect(
      transport.calls.every((entry) =>
        [
          'layout.inspect',
          'layout.rules',
          'layout.rule_glyphs',
          'glyphs.cell',
        ].includes(entry.method),
      ),
    ).toBe(true)
  })

  it('requests bounded continuation pages against the inspected generation', async () => {
    const transport = createMockTransport({
      'layout.inspect': () => inspection(),
      'layout.rules': ({ offset }) =>
        rules({
          total: 12,
          next_offset: offset === 0 ? 10 : null,
          items: [
            {
              text: offset === 0 ? 'first rule' : 'last rule',
              glyphs: [],
              truncated: false,
            },
          ],
        }),
    })
    setTransport(transport)
    render(
      <LookupInspector
        doc={1}
        master="m1"
        generation={3}
        onChanged={vi.fn()}
      />,
    )
    await screen.findByText('first rule')
    fireEvent.click(screen.getByRole('button', { name: 'Next rules' }))
    expect(await screen.findByText('last rule')).toBeVisible()
    expect(transport.calls).toContainEqual({
      method: 'layout.rules',
      params: {
        doc: 1,
        table: 'GSUB',
        lookup: 0,
        generation: 3,
        offset: 10,
        limit: 10,
      },
    })
    expect(screen.queryByText('first rule')).not.toBeInTheDocument()
  })

  it('drops delayed rule pages after a document revision changes', async () => {
    let finish: (value: LayoutRules) => void = () => undefined
    setTransport(
      createMockTransport({
        'layout.inspect': ({ generation }) =>
          inspection({ generation: generation ?? 3 }),
        'layout.rules': ({ generation }) =>
          generation === 3
            ? new Promise<LayoutRules>((resolve) => {
                finish = resolve
              })
            : rules({
                generation: 4,
                items: [{ text: 'current rule', glyphs: [], truncated: false }],
              }),
      }),
    )
    const view = render(
      <LookupInspector
        doc={1}
        master="m1"
        generation={3}
        onChanged={vi.fn()}
      />,
    )
    await waitFor(() =>
      expect(screen.getByRole('combobox', { name: 'Lookup' })).toBeVisible(),
    )
    view.rerender(
      <LookupInspector
        doc={1}
        master="m2"
        generation={4}
        onChanged={vi.fn()}
      />,
    )
    expect(await screen.findByText('current rule')).toBeVisible()
    await act(async () => finish(rules()))
    expect(screen.queryByText('sub a by a.alt;')).not.toBeInTheDocument()
  })

  it('retries a failed inspection without showing stale rules', async () => {
    let failed = true
    setTransport(
      createMockTransport({
        'layout.inspect': () => {
          if (failed) throw new Error('lost')
          return inspection()
        },
        'layout.rules': () => rules({ items: [] }),
      }),
    )
    render(
      <LookupInspector
        doc={1}
        master="m1"
        generation={3}
        onChanged={vi.fn()}
      />,
    )
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Could not load the original layout.',
    )
    failed = false
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }))
    expect(
      await screen.findByRole('combobox', { name: 'Lookup' }),
    ).toBeVisible()
  })
})
