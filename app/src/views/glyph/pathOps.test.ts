import { describe, expect, it } from 'vitest'

import type { LayerTarget } from '../../editor/pointCommands'
import { selectionOf } from '../../editor/selection'
import { makeContour, makePoint } from '../../test/fontFixtures'

import {
  alignComponentsCommand,
  breakPoint,
  DEFAULT_SIMPLIFY,
  joinEnds,
  onlyNode,
  openCornerCommand,
  openCornerDepth,
  openCornerPlan,
  pathOpCommand,
  reconnectCommand,
  reconnectPair,
  scopedContours,
  startPoint,
  tidyCommand,
} from './pathOps'

const target: LayerTarget = { doc: 1, glyph: 'A', layer: 'layer-1' }

const closed = makeContour([
  makePoint(0, 0),
  makePoint(100, 0),
  makePoint(100, 100),
])
const open = makeContour(
  [makePoint(0, 0), makePoint(50, 0), makePoint(90, 0)],
  false,
)
const otherOpen = makeContour([makePoint(200, 0), makePoint(300, 0)], false)

describe('path operations', () => {
  it('sends a whole-layer operation as one PathOp', () => {
    expect(pathOpCommand(target, 'RemoveOverlap')).toEqual({
      PathOp: { glyph: 'A', layer: 'layer-1', op: 'RemoveOverlap' },
    })
    expect(pathOpCommand(target, { Simplify: DEFAULT_SIMPLIFY })).toMatchObject(
      { PathOp: { op: { Simplify: { error: 0.1 } } } },
    )
  })

  it('makes Tidy one undo entry over both canonical operations', () => {
    expect(tidyCommand(target)).toEqual({
      Batch: [
        { PathOp: { glyph: 'A', layer: 'layer-1', op: 'CanonicalStart' } },
        { PathOp: { glyph: 'A', layer: 'layer-1', op: 'CanonicalOrder' } },
      ],
    })
  })
})

describe('selection scope', () => {
  it('sets a start only on a closed contour and an existing node', () => {
    const selection = selectionOf([{ kind: 'node', contour: 0, point: 0 }])
    expect(startPoint([closed], selection)).toEqual({ contour: 0, point: 0 })
    expect(startPoint([open], selection)).toBeNull()
    expect(startPoint([], selection)).toBeNull()
  })
  it('falls back to every contour when nothing is selected', () => {
    expect(scopedContours([closed, open], selectionOf([]))).toEqual([0, 1])
  })

  it('names only the contours the selection touches', () => {
    const selection = selectionOf([
      { kind: 'node', contour: 1, point: 0 },
      { kind: 'segment', contour: 1, segment: 0 },
    ])
    expect(scopedContours([closed, open], selection)).toEqual([1])
  })

  it('reports the single selected node and nothing for a pair', () => {
    expect(
      onlyNode(selectionOf([{ kind: 'node', contour: 0, point: 2 }])),
    ).toEqual({ contour: 0, point: 2 })
    expect(
      onlyNode(
        selectionOf([
          { kind: 'node', contour: 0, point: 1 },
          { kind: 'node', contour: 0, point: 2 },
        ]),
      ),
    ).toBeNull()
  })
})

describe('join', () => {
  it('orders the pair so `a` is the end of its contour', () => {
    const selection = selectionOf([
      { kind: 'node', contour: 1, point: 0 },
      { kind: 'node', contour: 0, point: 2 },
    ])
    expect(joinEnds([open, otherOpen], selection)).toEqual({
      a: { contour: 0, point: 2 },
      b: { contour: 1, point: 0 },
    })
  })

  it('reconnects the two ends of the same contour', () => {
    const selection = selectionOf([
      { kind: 'node', contour: 0, point: 0 },
      { kind: 'node', contour: 0, point: 2 },
    ])
    expect(joinEnds([open], selection)).toEqual({
      a: { contour: 0, point: 2 },
      b: { contour: 0, point: 0 },
    })
  })

  it('joins two starts and rejects selections containing interior nodes', () => {
    const starts = [
      { kind: 'node' as const, contour: 0, point: 0 },
      { kind: 'node' as const, contour: 1, point: 0 },
    ]
    expect(joinEnds([open, otherOpen], selectionOf(starts))).toEqual({
      a: { contour: 0, point: 0 },
      b: { contour: 1, point: 0 },
    })
    expect(
      joinEnds(
        [open, otherOpen],
        selectionOf([...starts, { kind: 'node', contour: 0, point: 1 }]),
      ),
    ).toBeNull()
  })

  it('refuses an interior node and a closed contour', () => {
    const selection = selectionOf([
      { kind: 'node', contour: 0, point: 1 },
      { kind: 'node', contour: 1, point: 0 },
    ])
    expect(joinEnds([open, otherOpen], selection)).toBeNull()
    expect(
      joinEnds(
        [closed, otherOpen],
        selectionOf([
          { kind: 'node', contour: 0, point: 0 },
          { kind: 'node', contour: 1, point: 0 },
        ]),
      ),
    ).toBeNull()
  })
})

describe('break', () => {
  it('opens a closed contour at any node', () => {
    expect(
      breakPoint(
        [closed],
        selectionOf([{ kind: 'node', contour: 0, point: 0 }]),
      ),
    ).toEqual({ contour: 0, point: 0 })
  })

  it('splits an open contour only at an interior node', () => {
    expect(
      breakPoint([open], selectionOf([{ kind: 'node', contour: 0, point: 1 }])),
    ).toEqual({ contour: 0, point: 1 })
    expect(
      breakPoint([open], selectionOf([{ kind: 'node', contour: 0, point: 0 }])),
    ).toBeNull()
  })
})

describe('open corner', () => {
  const chevron = makeContour([
    makePoint(0, 0),
    makePoint(100, 0),
    makePoint(50, 60),
    makePoint(100, 120),
    makePoint(0, 120),
  ])

  it('picks a setback bounded by the em and by the shorter adjacent chord', () => {
    // Both chords are about 78 units, so a quarter of that wins over 1000/50.
    expect(openCornerDepth(chevron, 2, 1000)).toBeCloseTo(19.53, 2)
    // A long corner is bounded by the em instead.
    const wide = makeContour([
      makePoint(0, 0),
      makePoint(1000, 0),
      makePoint(1000, 1000),
    ])
    expect(openCornerDepth(wide, 1, 1000)).toBe(20)
    expect(openCornerDepth(wide, 1, 2048)).toBe(40.96)
  })

  it('needs exactly one on-curve node with a segment on either side', () => {
    const plan = openCornerPlan(
      [chevron],
      selectionOf([{ kind: 'node', contour: 0, point: 2 }]),
      1000,
    )
    expect(plan).toMatchObject({ point: { contour: 0, point: 2 } })
    expect(plan?.depth).toBeGreaterThan(0)
    expect(
      openCornerPlan(
        [chevron],
        selectionOf([
          { kind: 'node', contour: 0, point: 1 },
          { kind: 'node', contour: 0, point: 2 },
        ]),
        1000,
      ),
    ).toBeNull()
    // The end of an open contour has only one segment.
    const path = makeContour(
      [makePoint(0, 0, 'Move'), makePoint(100, 0), makePoint(100, 100)],
      false,
    )
    expect(
      openCornerPlan(
        [path],
        selectionOf([{ kind: 'node', contour: 0, point: 2 }]),
        1000,
      ),
    ).toBeNull()
  })

  it('refuses a node that already carries a live smart corner', () => {
    const smart = makeContour([
      makePoint(0, 0),
      { ...makePoint(100, 0), corner_radius: 12 },
      makePoint(100, 100),
    ])
    expect(
      openCornerPlan(
        [smart],
        selectionOf([{ kind: 'node', contour: 0, point: 1 }]),
        1000,
      ),
    ).toBeNull()
  })

  it('builds the command the kernel validates', () => {
    expect(openCornerCommand(target, { contour: 0, point: 2 }, 12.5)).toEqual({
      OpenCorner: {
        glyph: 'A',
        layer: 'layer-1',
        point: { contour: 0, point: 2 },
        depth: 12.5,
      },
    })
  })
})

describe('reconnect nodes', () => {
  const opened = makeContour([
    makePoint(0, 0),
    makePoint(100, 0),
    makePoint(60, 40),
    makePoint(100, 120),
    makePoint(0, 120),
  ])

  it('orders the two nodes along the path, whatever the selection order', () => {
    const forward = reconnectPair(
      [opened],
      selectionOf([
        { kind: 'node', contour: 0, point: 1 },
        { kind: 'node', contour: 0, point: 2 },
      ]),
    )
    expect(forward).toEqual({
      a: { contour: 0, point: 1 },
      b: { contour: 0, point: 2 },
    })
    const reversed = reconnectPair(
      [opened],
      selectionOf([
        { kind: 'node', contour: 0, point: 2 },
        { kind: 'node', contour: 0, point: 1 },
      ]),
    )
    expect(reversed).toEqual(forward)
  })

  it('wraps across the closing seam of a closed contour', () => {
    expect(
      reconnectPair(
        [opened],
        selectionOf([
          { kind: 'node', contour: 0, point: 0 },
          { kind: 'node', contour: 0, point: 4 },
        ]),
      ),
    ).toEqual({ a: { contour: 0, point: 4 }, b: { contour: 0, point: 0 } })
  })

  it('refuses nodes that are not one segment apart or not one contour', () => {
    expect(
      reconnectPair(
        [opened],
        selectionOf([
          { kind: 'node', contour: 0, point: 0 },
          { kind: 'node', contour: 0, point: 2 },
        ]),
      ),
    ).toBeNull()
    expect(
      reconnectPair(
        [opened, closed],
        selectionOf([
          { kind: 'node', contour: 0, point: 0 },
          { kind: 'node', contour: 1, point: 0 },
        ]),
      ),
    ).toBeNull()
    expect(
      reconnectPair(
        [opened],
        selectionOf([{ kind: 'node', contour: 0, point: 0 }]),
      ),
    ).toBeNull()
    expect(
      reconnectPair(
        [opened],
        selectionOf([
          { kind: 'node', contour: 0, point: 1 },
          { kind: 'handle', contour: 0, point: 2 },
        ]),
      ),
    ).toBeNull()
  })

  it('builds the command the kernel validates', () => {
    expect(
      reconnectCommand(
        target,
        { contour: 0, point: 1 },
        { contour: 0, point: 2 },
      ),
    ).toEqual({
      ReconnectNodes: {
        glyph: 'A',
        layer: 'layer-1',
        a: { contour: 0, point: 1 },
        b: { contour: 0, point: 2 },
      },
    })
  })
})

describe('component alignment', () => {
  it('names the layer whose auto-aligned components are placed', () => {
    expect(alignComponentsCommand(target)).toEqual({
      AlignComponents: { glyph: 'A', layer: 'layer-1' },
    })
  })
})
