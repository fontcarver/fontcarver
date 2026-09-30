import { describe, expect, it } from 'vitest'

import type { LayerTarget } from '../../editor/pointCommands'
import { selectionOf } from '../../editor/selection'
import { makeContour, makePoint } from '../../test/fontFixtures'

import {
  breakPoint,
  DEFAULT_SIMPLIFY,
  joinEnds,
  onlyNode,
  pathOpCommand,
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
