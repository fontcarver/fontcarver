import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it } from 'vitest'

import {
  setTransport,
  type GlyphDetail,
  type GlyphListResult,
} from '../../../api'
import { i18n } from '../../../i18n'
import {
  makeCell,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
} from '../../../test/fontFixtures'
import { createMockTransport, noChange } from '../../../test/mockTransport'

import { MarkCloudPreview } from './MarkCloudPreview'

const base = makeLayer({ anchors: [{ name: 'top', x: 300, y: 700 }] })
const props = {
  doc: 1,
  gid: 42,
  baseName: 'a',
  master: 'm1',
  outline: makeLayerOutline(base),
  path: 'M0 0L600 0L300 700Z',
  upem: 1000,
}
const mark = makeGlyphDetail(
  makeGlyph({
    name: 'acutecomb',
    category: 'Mark',
    layers: [makeLayer({ anchors: [{ name: '_top', x: 100, y: 500 }] })],
  }),
  'M90 500L180 620L150 620Z',
)
const markCell = makeCell({
  gid: 7,
  name: 'acutecomb',
  category: 'Mark',
  bbox: [90, 500, 180, 620],
})

afterEach(() => setTransport(undefined))

describe('matching mark preview', () => {
  it('draws the base and matching mark with the attachment translation', async () => {
    const mock = createMockTransport({
      'glyphs.list': () => ({ total: 1, items: [markCell] }),
      'glyphs.detail': () => mark,
    })
    setTransport(mock)
    render(<MarkCloudPreview {...props} />)
    const preview = await screen.findByRole('img')
    expect(preview.querySelector('path')?.getAttribute('d')).toBe(props.path)
    expect(preview.querySelector('[data-mark]')).toHaveAttribute(
      'transform',
      'translate(200 200)',
    )
    expect(mock.calls[0]?.params).toMatchObject({
      master: 'm1',
      filter: { categories: ['Mark'] },
      limit: 24,
      include_paths: true,
    })
  })

  it('does not fetch without a base anchor', () => {
    const mock = createMockTransport()
    setTransport(mock)
    render(
      <MarkCloudPreview
        {...props}
        outline={makeLayerOutline(
          makeLayer({ anchors: [{ name: '_top', x: 0, y: 0 }] }),
        )}
      />,
    )
    expect(
      screen.getByText(i18n.t('panel.anchors.cloud.noAnchor')),
    ).toBeVisible()
    expect(mock.calls).toHaveLength(0)
  })

  it('does not preview a base fallback from a different master', () => {
    const mock = createMockTransport()
    setTransport(mock)
    render(<MarkCloudPreview {...props} master="m2" />)
    expect(
      screen.getByText(i18n.t('panel.anchors.cloud.noMaster')),
    ).toBeVisible()
    expect(mock.calls).toHaveLength(0)
  })

  it('reports failures and retries the page', async () => {
    const user = userEvent.setup()
    let failed = true
    setTransport(
      createMockTransport({
        'glyphs.list': () => {
          if (failed) throw new Error('offline')
          return { total: 1, items: [markCell] }
        },
        'glyphs.detail': () => mark,
      }),
    )
    render(<MarkCloudPreview {...props} />)
    expect(await screen.findByRole('alert')).toHaveTextContent(
      i18n.t('panel.anchors.cloud.error'),
    )
    failed = false
    await user.click(
      screen.getByRole('button', { name: i18n.t('panel.anchors.cloud.retry') }),
    )
    expect(await screen.findByRole('img')).toBeVisible()
    expect(screen.queryByRole('alert')).toBeNull()
  })

  it('discards old document responses without fetching their details', async () => {
    let resolve!: (result: GlyphListResult) => void
    const oldPage = new Promise<GlyphListResult>((done) => {
      resolve = done
    })
    const mock = createMockTransport({
      'glyphs.list': ({ doc }) =>
        doc === 1 ? oldPage : { total: 0, items: [] },
      'glyphs.detail': () => mark,
    })
    setTransport(mock)
    const view = render(<MarkCloudPreview {...props} />)
    view.rerender(<MarkCloudPreview {...props} doc={2} />)
    await screen.findByText(i18n.t('panel.anchors.cloud.noMatches'))
    await act(async () => resolve({ total: 1, items: [markCell] }))
    expect(screen.queryByRole('img')).toBeNull()
    expect(mock.calls.some((entry) => entry.method === 'glyphs.detail')).toBe(
      false,
    )
  })

  it('limits in-flight detail requests and keeps successful candidates on partial failure', async () => {
    let release!: (detail: GlyphDetail) => void
    const pending = new Promise<GlyphDetail>((done) => {
      release = done
    })
    const cells = Array.from({ length: 12 }, (_, index) => ({
      ...markCell,
      gid: index + 1,
      name: `mark${index}`,
    }))
    const mock = createMockTransport({
      'glyphs.list': () => ({ total: 12, items: cells }),
      'glyphs.detail': ({ gid }) => {
        if (gid === 12) throw new Error('missing')
        return pending
      },
    })
    setTransport(mock)
    render(<MarkCloudPreview {...props} />)
    await waitFor(() =>
      expect(
        mock.calls.filter((entry) => entry.method === 'glyphs.detail'),
      ).toHaveLength(4),
    )
    await act(async () => release(mark))
    expect(await screen.findAllByRole('img')).toHaveLength(11)
    expect(screen.getByRole('alert')).toHaveTextContent(
      i18n.t('panel.anchors.cloud.partial'),
    )
  })

  it('changes bounded candidate pages and refreshes marks after a document edit', async () => {
    const user = userEvent.setup()
    const mock = createMockTransport({
      'glyphs.list': () => ({ total: 25, items: [markCell] }),
      'glyphs.detail': () => mark,
    })
    setTransport(mock)
    render(<MarkCloudPreview {...props} />)
    await screen.findByRole('img')
    await user.click(
      screen.getByRole('button', { name: i18n.t('panel.anchors.cloud.next') }),
    )
    await screen.findByRole('img')
    expect(
      mock.calls.filter((entry) => entry.method === 'glyphs.list')[1]?.params,
    ).toMatchObject({ offset: 24 })
    act(() =>
      mock.emit({
        doc: 1,
        change: noChange([7]),
        generation: 1,
        undo_label: null,
        redo_label: null,
        dirty: true,
      }),
    )
    await waitFor(() =>
      expect(
        mock.calls.filter((entry) => entry.method === 'glyphs.list'),
      ).toHaveLength(3),
    )
  })

  it('abandons queued detail requests when the preview closes', async () => {
    let release!: (detail: GlyphDetail) => void
    const pending = new Promise<GlyphDetail>((done) => {
      release = done
    })
    const mock = createMockTransport({
      'glyphs.list': () => ({
        total: 12,
        items: Array.from({ length: 12 }, (_, index) => ({
          ...markCell,
          gid: index + 1,
        })),
      }),
      'glyphs.detail': () => pending,
    })
    setTransport(mock)
    const view = render(<MarkCloudPreview {...props} />)
    await waitFor(() =>
      expect(
        mock.calls.filter((entry) => entry.method === 'glyphs.detail'),
      ).toHaveLength(4),
    )
    view.unmount()
    await act(async () => release(mark))
    expect(
      mock.calls.filter((entry) => entry.method === 'glyphs.detail'),
    ).toHaveLength(4)
  })

  it('updates base-anchor placement locally without refetching marks', async () => {
    const mock = createMockTransport({
      'glyphs.list': () => ({ total: 1, items: [markCell] }),
      'glyphs.detail': () => mark,
    })
    setTransport(mock)
    const view = render(<MarkCloudPreview {...props} />)
    await screen.findByRole('img')
    act(() =>
      mock.emit({
        doc: 1,
        change: noChange([42]),
        generation: 1,
        undo_label: null,
        redo_label: null,
        dirty: true,
      }),
    )
    view.rerender(
      <MarkCloudPreview
        {...props}
        outline={{
          ...props.outline,
          anchors: [{ name: 'top', x: 325, y: 725 }],
        }}
      />,
    )
    expect(
      screen.getByRole('img').querySelector('[data-mark]'),
    ).toHaveAttribute('transform', 'translate(225 225)')
    expect(mock.calls).toHaveLength(2)
  })

  it('returns to an available page when mark deletion removes the last page', async () => {
    const user = userEvent.setup()
    let total = 25
    const mock = createMockTransport({
      'glyphs.list': ({ offset }) => ({
        total,
        items: offset < total ? [markCell] : [],
      }),
      'glyphs.detail': () => mark,
    })
    setTransport(mock)
    render(<MarkCloudPreview {...props} />)
    await screen.findByRole('img')
    await user.click(
      screen.getByRole('button', { name: i18n.t('panel.anchors.cloud.next') }),
    )
    await screen.findByRole('img')
    total = 20
    act(() =>
      mock.emit({
        doc: 1,
        change: noChange([7]),
        generation: 1,
        undo_label: null,
        redo_label: null,
        dirty: true,
      }),
    )
    expect(await screen.findByRole('img')).toBeVisible()
    expect(
      screen.getByRole('button', {
        name: i18n.t('panel.anchors.cloud.previous'),
      }),
    ).toBeDisabled()
    expect(
      mock.calls.filter((entry) => entry.method === 'glyphs.list').at(-1)
        ?.params,
    ).toMatchObject({ offset: 0 })
  })

  it('shares the four-detail limit across stale and current generations', async () => {
    let release!: (detail: GlyphDetail) => void
    const pending = new Promise<GlyphDetail>((done) => {
      release = done
    })
    const mock = createMockTransport({
      'glyphs.list': () => ({
        total: 4,
        items: Array.from({ length: 4 }, (_, index) => ({
          ...markCell,
          gid: index + 1,
        })),
      }),
      'glyphs.detail': ({ doc }) => (doc === 1 ? pending : mark),
    })
    setTransport(mock)
    const view = render(<MarkCloudPreview {...props} />)
    await waitFor(() =>
      expect(
        mock.calls.filter((entry) => entry.method === 'glyphs.detail'),
      ).toHaveLength(4),
    )
    view.rerender(<MarkCloudPreview {...props} doc={2} />)
    await waitFor(() =>
      expect(
        mock.calls.filter((entry) => entry.method === 'glyphs.list'),
      ).toHaveLength(2),
    )
    expect(
      mock.calls.filter((entry) => entry.method === 'glyphs.detail'),
    ).toHaveLength(4)
    await act(async () => release(mark))
    expect(await screen.findAllByRole('img')).toHaveLength(4)
    expect(
      mock.calls.filter((entry) => entry.method === 'glyphs.detail'),
    ).toHaveLength(8)
  })
})
