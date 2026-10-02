import { createRef } from 'react'
import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type ApplyParams } from '../../api'
import { Viewport } from '../../canvas/Viewport'
import type { EditorSource } from '../../editor/editorController'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import {
  makeContour,
  makeLayer,
  makeLayerOutline,
  makePoint,
} from '../../test/fontFixtures'
import { createMockTransport, noChange } from '../../test/mockTransport'

import {
  AUDIT_DEBOUNCE_MS,
  AUDIT_MAX_WAIT_MS,
  OutlineAudit,
  type OutlineAuditHandle,
} from './OutlineAudit'
import { DEFAULT_OUTLINE_AUDIT } from './outlineAudit'
import { useOutlineAuditStore } from './outlineAuditStore'

const outline = makeLayerOutline(
  makeLayer({
    contours: [
      makeContour([
        makePoint(10.2, 20.6),
        makePoint(100.7, 20.2),
        makePoint(100, 200),
      ]),
    ],
  }),
)
const source: EditorSource = {
  doc: 1,
  gid: 42,
  glyph: 'a',
  outline,
  master: 'm1',
  guides: [],
  metrics: {
    upem: 1000,
    width: 600,
    lsb: 10.2,
    rsb: 499.3,
    ascender: 800,
    descender: -200,
    xHeight: 500,
    capHeight: 700,
    italicAngle: 0,
    verticalOrigin: null,
    verticalWidth: null,
  },
  curves: 'cubic',
  componentPoints: [],
}

beforeEach(() => {
  useEditorStore.setState(initialEditorState())
  // The per-check switches persist between sessions, so a test that turns one
  // off has to put it back or it would silently disable the next test's fixes.
  globalThis.localStorage?.clear()
  useOutlineAuditStore.setState({ settings: { ...DEFAULT_OUTLINE_AUDIT } })
})
afterEach(() => setTransport(undefined))

async function show(props: Partial<Parameters<typeof OutlineAudit>[0]> = {}) {
  const user = userEvent.setup()
  const viewport = new Viewport()
  viewport.offset = { x: 100, y: 300 }
  const onReload = vi.fn()
  const view = render(
    <OutlineAudit
      source={source}
      viewport={viewport}
      onReload={onReload}
      {...props}
    />,
  )
  await user.click(screen.getByText(/Outline audit ·/))
  return { user, onReload, viewport, ...view }
}

describe('outline audit controls', () => {
  it('shows hover details and updates marker position after pan without changing the source', async () => {
    const handleRef = createRef<OutlineAuditHandle>()
    const { viewport } = await show({ handleRef })
    const marker = screen.getByRole('button', {
      name: 'Fractional coordinates at 10.2, 20.6',
    })
    expect(marker).toHaveAttribute(
      'title',
      expect.stringContaining('nearest font unit'),
    )
    expect(marker.style.left).toBe('110.2px')
    viewport.pan(40, 10)
    act(() => handleRef.current?.refresh())
    expect(marker.style.left).toBe('150.2px')
  })

  it('hides only disabled check markers and disables Fix All when nothing fixable is shown', async () => {
    const { user } = await show()
    await user.click(screen.getByLabelText('Fractional coordinates'))
    expect(
      screen.queryByRole('button', { name: /Fractional coordinates at/ }),
    ).toBeNull()
    expect(screen.getByRole('button', { name: 'Fix All' })).toBeDisabled()
    expect(screen.getByText('No issues in the enabled checks.')).toBeVisible()
  })

  it('applies all displayed fixes in one gesture after refreshing each pass', async () => {
    let repaired = false
    const transport = createMockTransport({
      'glyphs.outline': () =>
        repaired
          ? {
              ...outline,
              contours: outline.contours.map((contour) => ({
                ...contour,
                points: contour.points.map((point) => ({
                  ...point,
                  x: Math.round(point.x),
                  y: Math.round(point.y),
                })),
              })),
            }
          : outline,
      'doc.edit_begin': () => 7,
      'doc.edit_apply': () => {
        repaired = true
        return noChange([42])
      },
      'doc.edit_end': () => null,
    })
    setTransport(transport)
    const { user, onReload } = await show()
    await user.click(screen.getByRole('button', { name: 'Fix All' }))
    await waitFor(() => expect(onReload).toHaveBeenCalledOnce())
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'glyphs.outline',
      'doc.edit_begin',
      'doc.edit_apply',
      'glyphs.outline',
      'doc.edit_end',
    ])
    const applied = transport.calls[2]?.params as ApplyParams
    expect('Batch' in applied.command && applied.command.Batch).toHaveLength(2)
  })

  it('refuses a stale source before applying and reloads it', async () => {
    const transport = createMockTransport({
      'glyphs.outline': () => ({ ...outline, contours: [] }),
    })
    setTransport(transport)
    const { user, onReload } = await show()
    await user.click(screen.getAllByRole('button', { name: 'Fix' })[0]!)
    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent(
        'The outline changed',
      ),
    )
    expect(transport.calls.some((entry) => entry.method === 'doc.apply')).toBe(
      false,
    )
    expect(onReload).toHaveBeenCalledOnce()
  })

  it('reports a failed mutation and permits retry after reconciliation', async () => {
    setTransport(
      createMockTransport({
        'glyphs.outline': () => outline,
        'doc.edit_begin': () => 7,
        'doc.edit_apply': () => {
          throw new Error('Refused audit repair')
        },
        'doc.edit_cancel': () => noChange(),
      }),
    )
    const { user, onReload } = await show()
    await user.click(screen.getByRole('button', { name: 'Fix All' }))
    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent(
        'Refused audit repair',
      ),
    )
    expect(onReload).toHaveBeenCalledOnce()
    expect(screen.getByRole('button', { name: 'Fix All' })).toBeEnabled()
  })

  it('keeps auditing a live gesture but refuses to repair mid-drag', async () => {
    useEditorStore.setState({ editing: true })
    await show()
    // The markers stay: row 35 wants the audit on while the outline moves.
    expect(
      screen.getAllByRole('button', { name: /Fractional coordinates at/ }),
    ).not.toHaveLength(0)
    // Repairs are not applied against geometry the kernel has not accepted.
    expect(screen.getByRole('button', { name: 'Fix All' })).toBeDisabled()
    for (const fix of screen.getAllByRole('button', { name: 'Fix' }))
      expect(fix).toBeDisabled()
  })

  it('follows the gesture geometry once it settles, not the fetched outline', async () => {
    vi.useFakeTimers()
    try {
      useEditorStore.setState({ editing: true })
      const viewport = new Viewport()
      viewport.offset = { x: 100, y: 300 }
      render(
        <OutlineAudit source={source} viewport={viewport} onReload={vi.fn()} />,
      )
      // A drag moved the fractional point to a whole number, so its marker has
      // to go away once the geometry stops moving.
      act(() => {
        useEditorStore.setState({
          preview: {
            contours: [
              makeContour([
                makePoint(10, 20),
                makePoint(100.7, 20.2),
                makePoint(100, 200),
              ]),
            ],
            components: [],
            anchors: [],
          },
        })
      })
      act(() => {
        vi.advanceTimersByTime(AUDIT_DEBOUNCE_MS + 10)
      })
      expect(
        screen.queryByRole('button', {
          name: 'Fractional coordinates at 10.2, 20.6',
        }),
      ).toBeNull()
      expect(
        screen.getByRole('button', {
          name: 'Fractional coordinates at 100.7, 20.2',
        }),
      ).toBeVisible()
    } finally {
      vi.useRealTimers()
    }
  })

  it('keeps locked paths and glyphs read-only', async () => {
    const locked = { ...source, glyphLocked: true }
    const { rerender, viewport, onReload } = await show({ source: locked })
    expect(screen.getByRole('button', { name: 'Fix All' })).toBeDisabled()
    rerender(
      <OutlineAudit
        source={{
          ...source,
          editorMetadata: {
            'com.fontcarver.editor': { locked_paths: ['locked-path'] },
          },
          outline: {
            ...outline,
            contours: outline.contours.map((contour) => ({
              ...contour,
              id: 'locked-path',
            })),
          },
        }}
        viewport={viewport}
        onReload={onReload}
      />,
    )
    expect(screen.getByRole('button', { name: 'Fix All' })).toBeDisabled()
  })
})

describe('a drag that never pauses still refreshes the markers', () => {
  it('publishes on the ceiling even while every frame restarts the debounce', () => {
    vi.useFakeTimers()
    try {
      useEditorStore.setState({ editing: true })
      const viewport = new Viewport()
      viewport.offset = { x: 100, y: 300 }
      render(
        <OutlineAudit source={source} viewport={viewport} onReload={vi.fn()} />,
      )
      const moved = (x: number) => ({
        contours: [
          makeContour([
            makePoint(x, 20.6),
            makePoint(100.7, 20.2),
            makePoint(100, 200),
          ]),
        ],
        components: [],
        anchors: [],
      })
      // A frame every 40 ms: the 120 ms debounce alone would never fire.
      for (let frame = 1; frame <= 8; frame += 1) {
        act(() => {
          useEditorStore.setState({ preview: moved(10.2 + frame) })
        })
        act(() => {
          vi.advanceTimersByTime(40)
        })
      }
      expect(8 * 40).toBeGreaterThan(AUDIT_MAX_WAIT_MS)
      expect(AUDIT_MAX_WAIT_MS).toBeGreaterThan(AUDIT_DEBOUNCE_MS)
      // The marker followed the drag rather than staying on its first frame.
      expect(
        screen.queryByRole('button', {
          name: 'Fractional coordinates at 10.2, 20.6',
        }),
      ).toBeNull()
      expect(
        screen.getAllByRole('button', {
          name: /^Fractional coordinates at 1[0-9](\.\d+)?, 20\.6$/,
        }).length,
      ).toBeGreaterThan(0)
    } finally {
      vi.useRealTimers()
    }
  })
})
