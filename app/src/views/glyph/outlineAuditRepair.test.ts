import { afterEach, describe, expect, it } from 'vitest'

import { setTransport, type Command } from '../../api'
import {
  EditSession,
  immediateScheduler,
  pendingDocumentEdits,
} from '../../editor/editSession'
import {
  makeContour,
  makeLayer,
  makeLayerOutline,
  makePoint,
} from '../../test/fontFixtures'
import { createMockTransport, noChange } from '../../test/mockTransport'

import { DEFAULT_OUTLINE_AUDIT } from './outlineAudit'
import { OutlineAuditRepairError, repairAllOutline } from './outlineAuditRepair'

const original = [
  makeContour(
    [
      makePoint(10.2, 0, 'Move'),
      makePoint(100, 0),
      makePoint(100, 0),
      makePoint(100, 100),
    ],
    false,
  ),
]
const target = { doc: 401, glyph: 'a', layer: 'layer-1' }
const options = () => ({
  target,
  gid: 1,
  original,
  settings: { ...DEFAULT_OUTLINE_AUDIT },
  lockedPaths: new Set<string>(),
  label: 'Repair',
  isCurrent: () => true,
})
const outline = (contours = original) =>
  makeLayerOutline(makeLayer({ contours }))
afterEach(() => setTransport(undefined))

describe('Fix All repair gesture', () => {
  it('re-reads changed indexes and resolves combined structural defects in one gesture', async () => {
    const afterDelete = [
      makeContour(
        [makePoint(10.2, 0, 'Move'), makePoint(100, 0), makePoint(100, 100)],
        false,
      ),
    ]
    const afterClose = [
      makeContour(
        afterDelete[0]!.points.map((point) => ({
          ...point,
          kind: 'Line' as const,
        })),
      ),
    ]
    const clean = [
      makeContour(
        afterClose[0]!.points.map((point) => ({
          ...point,
          x: Math.round(point.x),
        })),
      ),
    ]
    const states = [original, afterDelete, afterClose, clean]
    let step = 0
    const commands: Command[] = []
    const transport = createMockTransport({
      'glyphs.outline': () => outline(states[step]),
      'doc.edit_begin': () => 17,
      'doc.edit_apply': ({ command }) => {
        commands.push(command)
        step += 1
        return noChange()
      },
      'doc.edit_end': () => null,
    })
    setTransport(transport)
    const running = repairAllOutline(options())
    expect(pendingDocumentEdits(target.doc)).toBeDefined()
    await running
    expect(commands).toMatchObject([
      { DeletePoints: { points: [{ contour: 0, point: 2 }] } },
      { JoinContours: { a: { contour: 0, point: 2 } } },
      { SetPointPosition: { point: { contour: 0, point: 0 }, x: 10 } },
    ])
    expect(
      transport.calls.filter((entry) => entry.method === 'doc.edit_begin'),
    ).toHaveLength(1)
    expect(transport.calls.at(-1)?.method).toBe('doc.edit_end')
    expect(step).toBe(3)
  })

  it('cancels a non-converging repair without issuing global undo', async () => {
    const transport = createMockTransport({
      'glyphs.outline': () => outline(),
      'doc.edit_begin': () => 17,
      'doc.edit_apply': () => noChange(),
      'doc.edit_cancel': () => noChange(),
    })
    setTransport(transport)
    await expect(repairAllOutline(options())).rejects.toEqual(
      new OutlineAuditRepairError('limit'),
    )
    expect(
      transport.calls.filter((entry) => entry.method === 'doc.edit_apply'),
    ).toHaveLength(1)
    expect(transport.calls.at(-1)?.method).toBe('doc.edit_cancel')
    expect(transport.calls.some((entry) => entry.method === 'doc.undo')).toBe(
      false,
    )
  })

  it('cancels on a rejected command and never closes another edit after a stale cancellation', async () => {
    const transport = createMockTransport({
      'glyphs.outline': () => outline(),
      'doc.edit_begin': () => 17,
      'doc.edit_apply': () => {
        throw new Error('Glyph locked')
      },
      'doc.edit_cancel': () => {
        throw new Error('InvalidEditToken')
      },
    })
    setTransport(transport)
    await expect(repairAllOutline(options())).rejects.toThrow(
      'InvalidEditToken',
    )
    expect(transport.calls.at(-1)?.method).toBe('doc.edit_cancel')
    expect(
      transport.calls.some(
        (entry) =>
          entry.method === 'doc.undo' || entry.method === 'doc.edit_end',
      ),
    ).toBe(false)
  })

  it('keeps disabled checks and locked contours out of every pass', async () => {
    const locked = [{ ...original[0]!, id: 'locked' }]
    const transport = createMockTransport({
      'glyphs.outline': () => outline(locked),
      'doc.edit_begin': () => 17,
      'doc.edit_end': () => null,
    })
    setTransport(transport)
    await repairAllOutline({
      ...options(),
      original: locked,
      lockedPaths: new Set(['locked']),
    })
    expect(
      transport.calls.some((entry) => entry.method === 'doc.edit_apply'),
    ).toBe(false)
    setTransport(
      createMockTransport({
        'glyphs.outline': () => outline(),
        'doc.edit_begin': () => 18,
        'doc.edit_end': () => null,
      }),
    )
    await repairAllOutline({
      ...options(),
      settings: {
        open: false,
        fractional: false,
        coincident: false,
        extrema: false,
      },
    })
  })

  it('refuses a stale initial snapshot before starting an edit', async () => {
    const transport = createMockTransport({
      'glyphs.outline': () => outline([]),
    })
    setTransport(transport)
    await expect(repairAllOutline(options())).rejects.toEqual(
      new OutlineAuditRepairError('stale'),
    )
    expect(transport.calls).toHaveLength(1)
  })

  it('cancels before a waiting newer gesture can begin during a fetch', async () => {
    let fetched!: () => void
    let release!: () => void
    const fetchedGate = new Promise<void>((resolve) => {
      fetched = resolve
    })
    const gate = new Promise<void>((resolve) => {
      release = resolve
    })
    let reads = 0
    const transport = createMockTransport({
      'glyphs.outline': async () => {
        reads += 1
        if (reads === 2) {
          fetched()
          await gate
        }
        return outline()
      },
      'doc.edit_begin': () => 17,
      'doc.edit_apply': () => noChange(),
      'doc.edit_cancel': () => noChange(),
      'doc.edit_end': () => null,
    })
    setTransport(transport)
    const repairing = repairAllOutline(options())
    const rejected = expect(repairing).rejects.toEqual(
      new OutlineAuditRepairError('stale'),
    )
    await fetchedGate
    const next = new EditSession(target.doc, 'Next', {
      scheduler: immediateScheduler,
    })
    release()
    await rejected
    await next.end()
    const methods = transport.calls.map((entry) => entry.method)
    expect(methods.indexOf('doc.edit_cancel')).toBeLessThan(
      methods.lastIndexOf('doc.edit_begin'),
    )
  })
})
