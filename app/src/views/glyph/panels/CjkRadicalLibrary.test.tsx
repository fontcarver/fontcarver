import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import {
  setTransport,
  type CjkRadicalLibrary as LibraryData,
} from '../../../api'
import {
  initialDocumentState,
  newDocView,
  useDocumentStore,
} from '../../../stores/documentStore'
import { makeCell } from '../../../test/fontFixtures'
import { createMockTransport } from '../../../test/mockTransport'

import { CjkRadicalLibrary } from './CjkRadicalLibrary'

/** Radicals 9 人/亻 and 75 木, as the kernel derives them from Unihan. */
function libraryData(components: LibraryData['components']): LibraryData {
  return {
    unihan_version: '17.0.0',
    radicals: [
      {
        number: 9,
        kangxi: 0x2f08,
        kangxi_glyph: null,
        forms: [
          { unicode: 0x4eba, strokes: 2, glyph: 'uni4EBA' },
          { unicode: 0x4ebb, strokes: 2, glyph: null },
        ],
        members: 9,
      },
      {
        number: 75,
        kangxi: 0x2f4a,
        kangxi_glyph: null,
        forms: [{ unicode: 0x6728, strokes: 4, glyph: 'uni6728' }],
        members: 3,
      },
    ],
    components,
    components_truncated: false,
  }
}

/** One kernel document event, as the transport delivers it. */
function edit(doc: number) {
  return {
    doc,
    generation: 9,
    change: {
      glyphs: [4],
      font_info: false,
      masters: false,
      layout: false,
      bitmaps: false,
      math: false,
      color: false,
      feature_source: false,
      glyph_order: false,
      metrics: false,
      layers: false,
      guides: false,
    },
    undo_label: 'Construct CJK components',
    redo_label: null,
    dirty: true,
  }
}

const tree = makeCell({
  name: 'uni6728',
  unicodes: [0x6728],
  width: 1000,
  path: 'M100 0L900 0L900 800L100 800Z',
  bbox: [100, 0, 900, 800],
})

beforeEach(() => useDocumentStore.setState(initialDocumentState()))
afterEach(() => setTransport(undefined))

describe('CJK radical library', () => {
  it('lists the pinned radicals with their forms and the font’s glyphs', async () => {
    const onPick = vi.fn()
    setTransport(
      createMockTransport({
        'cjk.radicals': () => libraryData([]),
        'glyphs.list': () => ({ total: 1, items: [tree] }),
      }),
    )
    render(
      <CjkRadicalLibrary
        doc={1}
        master="m1"
        glyph="uni6797"
        generation={0}
        onPick={onPick}
      />,
    )
    const user = userEvent.setup()
    // Only the forms this font has, until the switch is turned off.
    const present = await screen.findByRole('button', {
      name: /Insert Radical 9 · 2 strokes · U\+4EBA · uni4EBA · 9 glyphs/,
    })
    expect(
      screen.queryByRole('button', { name: /U\+4EBB/ }),
    ).not.toBeInTheDocument()
    await user.click(screen.getByLabelText('Only forms this font has'))
    // A form the font lacks is still listed, and picking it inserts the
    // character rather than a glyph name that does not exist.
    const missing = await screen.findByRole('button', {
      name: /Insert Radical 9 · 2 strokes · U\+4EBB · 9 glyphs/,
    })
    await user.click(present)
    expect(onPick).toHaveBeenLastCalledWith('[uni4EBA]')
    await user.click(missing)
    expect(onPick).toHaveBeenLastCalledWith('亻')
    // The source of the data is stated, with the pinned version.
    expect(screen.getByText(/Unicode 17\.0\.0 Unihan/)).toBeInTheDocument()
  })

  it('reloads when the document generation changes', async () => {
    const calls: number[] = []
    setTransport(
      createMockTransport({
        'cjk.radicals': () => {
          calls.push(calls.length)
          return libraryData(
            calls.length > 1
              ? [{ name: 'uni6728', uses: 2, unicode: 0x6728 }]
              : [],
          )
        },
        'glyphs.list': () => ({ total: 1, items: [tree] }),
      }),
    )
    const view = render(
      <CjkRadicalLibrary
        doc={1}
        master="m1"
        glyph="uni6797"
        generation={0}
        onPick={vi.fn()}
      />,
    )
    const user = userEvent.setup()
    await user.click(screen.getByRole('tab', { name: 'Font components' }))
    expect(
      await screen.findByText(/This font uses no components yet/),
    ).toBeInTheDocument()

    // The glyph panel passes the document generation down; a change to it must
    // refetch, because appending components changes what this list shows.
    act(() =>
      useDocumentStore.setState({
        byDoc: { 1: { ...newDocView(), generation: 7 } },
      }),
    )
    view.rerender(
      <CjkRadicalLibrary
        doc={1}
        master="m1"
        glyph="uni6797"
        generation={7}
        onPick={vi.fn()}
      />,
    )
    expect(
      await screen.findByRole('button', {
        name: /Insert U\+6728 · uni6728 · 2 component uses/,
      }),
    ).toBeInTheDocument()
    await waitFor(() => expect(calls.length).toBe(2))
  })

  it('refetches on a document event, and collapses a burst into one scan', async () => {
    const calls: number[] = []
    const transport = createMockTransport({
      'cjk.radicals': () => {
        calls.push(calls.length)
        return libraryData(
          calls.length > 1
            ? [{ name: 'uni6728', uses: 2, unicode: 0x6728 }]
            : [],
        )
      },
      'glyphs.list': () => ({ total: 1, items: [tree] }),
    })
    setTransport(transport)
    render(
      <CjkRadicalLibrary
        doc={1}
        master="m1"
        glyph="uni6797"
        generation={0}
        onPick={vi.fn()}
      />,
    )
    const user = userEvent.setup()
    await user.click(screen.getByRole('tab', { name: 'Font components' }))
    expect(
      await screen.findByText(/This font uses no components yet/),
    ).toBeInTheDocument()
    await waitFor(() => expect(calls.length).toBe(1))

    // The kernel's own event is enough; the generation prop may still be the
    // value the panel was rendered with when the command finished.
    act(() => transport.emit(edit(1)))
    expect(
      await screen.findByRole('button', {
        name: /Insert U\+6728 · uni6728 · 2 component uses/,
      }),
    ).toBeInTheDocument()
    expect(calls.length).toBe(2)

    // `cjk.radicals` scans every layer for component bases, so a continuous
    // gesture must not pay for that scan once per frame: a burst of events is
    // one rebuild.
    for (let index = 0; index < 8; index += 1)
      act(() => transport.emit(edit(1)))
    await waitFor(() => expect(calls.length).toBe(3))
    await new Promise((resolve) => setTimeout(resolve, 150))
    expect(calls.length).toBe(3)

    // An event for another document leaves this library alone.
    act(() => transport.emit(edit(2)))
    await new Promise((resolve) => setTimeout(resolve, 150))
    expect(calls.length).toBe(3)
  })

  it('follows the tab pattern: one tab stop, arrow keys and a named panel', async () => {
    setTransport(
      createMockTransport({
        'cjk.radicals': () => libraryData([]),
        'glyphs.list': () => ({ total: 1, items: [tree] }),
      }),
    )
    render(
      <CjkRadicalLibrary
        doc={1}
        master="m1"
        glyph="uni6797"
        generation={0}
        onPick={vi.fn()}
      />,
    )
    const radicals = screen.getByRole('tab', { name: 'Radicals' })
    const components = screen.getByRole('tab', { name: 'Font components' })
    const panel = screen.getByRole('tabpanel')
    // One tab stop for the strip, and the panel names the tab that owns it.
    expect(radicals).toHaveAttribute('tabindex', '0')
    expect(components).toHaveAttribute('tabindex', '-1')
    expect(radicals).toHaveAttribute('aria-controls', panel.id)
    expect(panel).toHaveAttribute('aria-labelledby', radicals.id)

    const user = userEvent.setup()
    radicals.focus()
    await user.keyboard('{ArrowRight}')
    expect(components).toHaveFocus()
    expect(components).toHaveAttribute('aria-selected', 'true')
    expect(screen.getByRole('tabpanel')).toHaveAttribute(
      'aria-labelledby',
      components.id,
    )
    // The ends wrap, and Home/End jump.
    await user.keyboard('{ArrowRight}')
    expect(screen.getByRole('tab', { name: 'Radicals' })).toHaveFocus()
    await user.keyboard('{End}')
    expect(screen.getByRole('tab', { name: 'Font components' })).toHaveFocus()
    await user.keyboard('{Home}')
    expect(screen.getByRole('tab', { name: 'Radicals' })).toHaveFocus()
  })

  it('reports a failed library without pretending the font has no radicals', async () => {
    setTransport(
      createMockTransport({
        'cjk.radicals': () => {
          throw new Error('kernel unavailable')
        },
        'glyphs.list': () => ({ total: 0, items: [] }),
      }),
    )
    render(
      <CjkRadicalLibrary
        doc={1}
        master="m1"
        glyph="uni6797"
        generation={0}
        onPick={vi.fn()}
      />,
    )
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Could not load the radical library.',
    )
  })
})
