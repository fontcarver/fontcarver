import { describe, expect, it, vi } from 'vitest'

import type { GlyphCell, GlyphId } from '../../api'
import { Viewport } from '../../canvas/Viewport'
import { FALLBACK_GLYPH_THEME as THEME } from '../glyph/glyphTheme'
import { drawTextGaps } from './drawTextGaps'
import type { TextCanvasRun } from './textCanvas'
import type { PositionedGlyph } from './textLayout'

/** A canvas context that records every style it is given and every text drawn. */
function context() {
  const fills: string[] = []
  const strokes: string[] = []
  const texts: string[] = []
  const base = {
    save: vi.fn(),
    restore: vi.fn(),
    setTransform: vi.fn(),
    translate: vi.fn(),
    fill: vi.fn(),
    fillRect: vi.fn(),
    stroke: vi.fn(),
    setLineDash: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    closePath: vi.fn(),
    arc: vi.fn(),
    measureText: vi.fn(() => ({ width: 12 })),
    fillText: vi.fn((text: string) => texts.push(text)),
    font: '',
    textAlign: '',
    textBaseline: '',
    globalAlpha: 1,
    lineWidth: 1,
  }
  Object.defineProperties(base, {
    fillStyle: {
      get: () => fills.at(-1) ?? '',
      set: (value: string) => void fills.push(value),
    },
    strokeStyle: {
      get: () => strokes.at(-1) ?? '',
      set: (value: string) => void strokes.push(value),
    },
  })
  return {
    ctx: base as unknown as CanvasRenderingContext2D,
    fills,
    strokes,
    texts,
    calls: base,
  }
}

function glyph(
  name: string,
  x: number,
  xAdvance: number,
  gid: number,
): PositionedGlyph {
  return {
    compiledGid: gid,
    documentGid: gid as GlyphId,
    name,
    editable: true,
    x,
    y: 0,
    xAdvance,
    yAdvance: 0,
    sourceByte: 0,
    sourceKind: 'literal',
  }
}

function cell(gid: number, width: number): [GlyphId, GlyphCell] {
  return [gid as GlyphId, { width } as GlyphCell]
}

function run(overrides: Partial<TextCanvasRun> = {}): TextCanvasRun {
  return {
    // "AV": A is 500 wide and was shaped 40 units tighter.
    glyphs: [glyph('A', 0, 460, 1), glyph('V', 460, 500, 2)],
    cells: new Map([cell(1, 500), cell(2, 500)]),
    activeIndex: null,
    activeOrigin: { x: 100, y: 50 },
    mode: 'text',
    unitsPerEm: 1000,
    fitBox: { x0: 0, x1: 1000, y0: 0, y1: 1000 },
    requestedScale: 1,
    onActivate: vi.fn(),
    onExitEdit: vi.fn(),
    onVisibleIndexes: vi.fn(),
    selectedGap: null,
    gaps: [{ index: 0, left: 'A', right: 'V', x: 460, reason: null }],
    ...overrides,
  }
}

describe('drawTextGaps', () => {
  it.each([
    [-40, 'warning'],
    [40, 'nodeSmooth'],
    [0, 'accent'],
    [null, 'accent'],
  ] as const)(
    'colours the selected boundary from its resolved value %s',
    (value, colour) => {
      const { ctx, fills, strokes, calls } = context()
      const viewport = new Viewport()
      drawTextGaps(
        ctx,
        viewport,
        run({ selectedGap: 0, selectedKerningValue: value }),
        THEME,
        [0],
      )
      expect(strokes).toContain(THEME[colour])
      expect(fills).toContain(THEME[colour])
      // The handle stays where the kernel put the boundary.
      const point = viewport.toScreen({ x: 360, y: -170 })
      expect(calls.moveTo).toHaveBeenCalledWith(point.x, point.y)
    },
  )

  it('colours every other boundary from the kerning the run applied', () => {
    const { ctx, fills, texts } = context()
    drawTextGaps(ctx, new Viewport(), run(), THEME, [0, 1])
    expect(fills).toContain(THEME.warning)
    // Colour is never the only signal: the value is printed beside the handle.
    expect(texts).toContain('-40')
  })

  it('leaves an unkerned boundary neutral and unlabelled', () => {
    const { ctx, fills, texts } = context()
    const neutral = run({
      glyphs: [glyph('A', 0, 500, 1), glyph('V', 500, 500, 2)],
      gaps: [{ index: 0, left: 'A', right: 'V', x: 500, reason: null }],
    })
    drawTextGaps(ctx, new Viewport(), neutral, THEME, [0, 1])
    expect(fills).toContain(THEME.metricStrong)
    expect(fills).not.toContain(THEME.warning)
    expect(texts).toEqual([])
  })

  it('stays neutral while the left glyph is still loading', () => {
    const { ctx, fills, texts } = context()
    drawTextGaps(ctx, new Viewport(), run({ cells: new Map() }), THEME, [0, 1])
    expect(fills).toContain(THEME.metricStrong)
    expect(texts).toEqual([])
  })

  it('rings the selected boundary when its winning rule is an exception', () => {
    const plain = context()
    drawTextGaps(
      plain.ctx,
      new Viewport(),
      run({ selectedGap: 0, selectedKerningValue: -40 }),
      THEME,
      [0],
    )
    expect(plain.calls.arc).not.toHaveBeenCalled()
    const marked = context()
    drawTextGaps(
      marked.ctx,
      new Viewport(),
      run({
        selectedGap: 0,
        selectedKerningValue: -40,
        selectedKerningException: true,
      }),
      THEME,
      [0],
    )
    expect(marked.calls.arc).toHaveBeenCalled()
  })

  it('rings every exception boundary, selected or not', () => {
    const plain = context()
    drawTextGaps(plain.ctx, new Viewport(), run(), THEME, [0, 1])
    expect(plain.calls.arc).not.toHaveBeenCalled()
    const marked = context()
    drawTextGaps(
      marked.ctx,
      new Viewport(),
      run({ kerningExceptions: new Set([0]) }),
      THEME,
      [0, 1],
    )
    expect(marked.calls.arc).toHaveBeenCalled()
    // A boundary that is not an exception stays unringed.
    const other = context()
    drawTextGaps(
      other.ctx,
      new Viewport(),
      run({ kerningExceptions: new Set([7]) }),
      THEME,
      [0, 1],
    )
    expect(other.calls.arc).not.toHaveBeenCalled()
  })

  it('measures the distance between the facing margins on the beam', () => {
    const { ctx, texts } = context()
    drawTextGaps(
      ctx,
      new Viewport(),
      run({
        beam: {
          height: 250,
          margins: new Map([
            ['A', { left: 10, right: 30 }],
            ['V', { left: 25, right: 5 }],
          ]),
        },
      }),
      THEME,
      [0, 1],
    )
    // 30 right margin + 25 left margin - 40 of applied kerning.
    expect(texts).toContain('15')
  })

  it('does not measure a beam through a glyph it could not measure', () => {
    const { ctx, texts } = context()
    drawTextGaps(
      ctx,
      new Viewport(),
      run({
        beam: {
          height: 250,
          margins: new Map([['A', { left: 10, right: 30 }]]),
        },
      }),
      THEME,
      [0, 1],
    )
    expect(texts).toEqual(['-40'])
  })

  it('covers everything outside the cut-off band with the canvas background', () => {
    const { ctx, fills, calls } = context()
    const viewport = new Viewport()
    // Put the whole em on screen so both sides of the band are visible.
    viewport.setSize(800, 600)
    viewport.scale = 0.5
    viewport.offset = { x: 0, y: 500 }
    drawTextGaps(
      ctx,
      viewport,
      run({ cutOff: { from: 0, to: 500 } }),
      THEME,
      [0, 1],
    )
    expect(fills).toContain(THEME.background)
    const top = viewport.toScreen({ x: 0, y: 450 }).y
    const bottom = viewport.toScreen({ x: 0, y: -50 }).y
    expect(calls.fillRect).toHaveBeenCalledWith(0, 0, viewport.width, top)
    expect(calls.fillRect).toHaveBeenCalledWith(
      0,
      bottom,
      viewport.width,
      viewport.height - bottom,
    )
  })

  it('draws nothing at all while a glyph is being edited', () => {
    const { ctx, calls } = context()
    drawTextGaps(
      ctx,
      new Viewport(),
      run({ mode: 'edit', cutOff: { from: 0, to: 500 } }),
      THEME,
      [0, 1],
    )
    expect(calls.save).not.toHaveBeenCalled()
    expect(calls.fillRect).not.toHaveBeenCalled()
  })
})
