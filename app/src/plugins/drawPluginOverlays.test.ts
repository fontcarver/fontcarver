import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { Viewport } from '../canvas/Viewport'
import type { EditorSource } from '../editor/editorTypes'
import {
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
} from '../test/fontFixtures'
import { buildScene } from '../views/glyph/glyphScene'
import { FALLBACK_GLYPH_THEME as theme } from '../views/glyph/glyphTheme'
import type { FilterTarget } from './contracts'
import { drawPluginOverlays } from './drawPluginOverlays'
import { useFilterPreview, type FilterPreview } from './filterSession'
import { usePluginStore } from './pluginStore'
import { boundsReporter, translateFilter } from './registry'

const scene = buildScene({
  detail: makeGlyphDetail(),
  cell: null,
  master: null,
  masterId: 'm1',
  unitsPerEm: 1000,
})
const source: EditorSource = {
  doc: 1,
  gid: 2,
  glyph: 'a',
  master: 'm1',
  outline: makeLayerOutline(makeLayer(), { bbox: [20, -30, 200, 400] }),
  metrics: scene.metrics,
  guides: [],
  curves: 'cubic',
  componentPoints: [],
}
const target: FilterTarget = {
  doc: 1,
  gid: 2,
  glyph: 'a',
  layer: 'layer-1',
  master: 'm1',
}

function preview(overrides: Partial<FilterPreview> = {}): FilterPreview {
  return {
    pluginId: translateFilter.manifest.id,
    target,
    generation: 3,
    plan: translateFilter.plan(target, { dx: 10, dy: 0 }, 'Translate'),
    overlays: [{ kind: 'path', path: 'M10 0L30 0L30 40Z' }],
    result: {
      generation: 3,
      glyph_count: 1,
      layer_count: 1,
      decomposed_layers: 0,
      approximated_layers: 0,
      approximated_sources: [],
      examples: [],
      change: null,
    },
    ...overrides,
  }
}

function canvas() {
  const original = { strokeStyle: '#original', lineWidth: 7, globalAlpha: 0.4 }
  const ctx = {
    ...original,
    save: vi.fn(),
    restore: vi.fn(() => Object.assign(ctx, original)),
    translate: vi.fn(),
    scale: vi.fn(),
    setLineDash: vi.fn(),
    strokeRect: vi.fn(),
    stroke: vi.fn(),
  }
  return ctx as unknown as CanvasRenderingContext2D
}

function install(id: string, enabled = true) {
  usePluginStore.setState({
    installed: { [id]: { enabled, version: '1.0.0' } },
  })
}

beforeEach(() => {
  usePluginStore.setState({ installed: {} })
  useFilterPreview.getState().cancel()
  vi.stubGlobal(
    'Path2D',
    class {
      constructor(readonly data: string) {}
    },
  )
})
afterEach(() => {
  useFilterPreview.getState().cancel()
  usePluginStore.setState({ installed: {} })
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

describe('bundled reporter drawing', () => {
  it('draws exact layer bounds in font coordinates, keeping strokes in CSS pixels', () => {
    install(boundsReporter.manifest.id)
    const ctx = canvas()
    const viewport = new Viewport()
    viewport.scale = 2
    viewport.dpr = 3
    viewport.offset = { x: 12, y: 450 }
    vi.mocked(ctx.strokeRect).mockImplementation(() => {
      expect(ctx.strokeStyle).toBe(theme.accent)
      expect(ctx.lineWidth).toBe(0.75)
      expect(ctx.globalAlpha).toBe(1)
    })
    drawPluginOverlays(ctx, viewport, {
      source,
      scene: { ...scene, bbox: { x0: 0, y0: 0, x1: 999, y1: 999 } },
      theme,
    })
    expect(ctx.translate).toHaveBeenCalledWith(12, 450)
    expect(ctx.scale).toHaveBeenCalledWith(2, -2)
    expect(ctx.strokeRect).toHaveBeenCalledWith(20, -30, 180, 430)
    expect(ctx.setLineDash).toHaveBeenCalledWith([])
    expect(ctx.restore).toHaveBeenCalledOnce()
    expect(ctx.strokeStyle).toBe('#original')
    expect(ctx.lineWidth).toBe(7)
    expect(ctx.globalAlpha).toBe(0.4)
  })

  it.each([false, undefined])(
    'does not draw disabled or uninstalled reporters (%s)',
    (enabled) => {
      if (enabled !== undefined) install(boundsReporter.manifest.id, enabled)
      const ctx = canvas()
      drawPluginOverlays(ctx, new Viewport(), { source, scene, theme })
      expect(ctx.strokeRect).not.toHaveBeenCalled()
      expect(ctx.save).not.toHaveBeenCalled()
    },
  )

  it('ignores unknown persisted plugin ids', () => {
    install('external.unknown')
    const ctx = canvas()
    drawPluginOverlays(ctx, new Viewport(), { source, scene, theme })
    expect(ctx.save).not.toHaveBeenCalled()
  })

  it('does not reuse scene bounds when the editable layer is empty', () => {
    install(boundsReporter.manifest.id)
    const ctx = canvas()
    drawPluginOverlays(ctx, new Viewport(), {
      source: { ...source, outline: { ...source.outline, bbox: null } },
      scene: { ...scene, bbox: { x0: 0, y0: 0, x1: 999, y1: 999 } },
      theme,
    })
    expect(ctx.strokeRect).not.toHaveBeenCalled()
  })

  it('suppresses all overlays without an editable source', () => {
    install(boundsReporter.manifest.id)
    const ctx = canvas()
    drawPluginOverlays(ctx, new Viewport(), { source: null, scene, theme })
    expect(ctx.save).not.toHaveBeenCalled()
  })

  it('restores the canvas even when a drawing operation fails', () => {
    install(boundsReporter.manifest.id)
    const ctx = canvas()
    vi.mocked(ctx.strokeRect).mockImplementation(() => {
      throw new Error('failed stroke')
    })
    expect(() =>
      drawPluginOverlays(ctx, new Viewport(), { source, scene, theme }),
    ).toThrow('failed stroke')
    expect(ctx.restore).toHaveBeenCalledOnce()
    expect(ctx.strokeStyle).toBe('#original')
  })
})

describe('filter preview drawing', () => {
  it('paints matching previews as dashed outlines and reuses parsed paths on repaint', () => {
    install(translateFilter.manifest.id)
    const construct = vi.fn()
    vi.stubGlobal(
      'Path2D',
      class {
        constructor(data: string) {
          construct(data)
        }
      },
    )
    useFilterPreview.setState({
      preview: preview({ overlays: [{ kind: 'path', path: 'M7 8L21 22Z' }] }),
    })
    const ctx = canvas()
    const viewport = new Viewport()
    viewport.scale = 2
    vi.mocked(ctx.stroke).mockImplementation(() => {
      expect(ctx.strokeStyle).toBe(theme.warning)
      expect(ctx.lineWidth).toBe(0.75)
    })
    drawPluginOverlays(ctx, viewport, { source, scene, theme })
    drawPluginOverlays(ctx, viewport, { source, scene, theme })
    expect(ctx.stroke).toHaveBeenCalledTimes(2)
    expect(ctx.setLineDash).toHaveBeenCalledWith([3, 2])
    expect(construct).toHaveBeenCalledExactlyOnceWith('M7 8L21 22Z')
    expect(ctx.restore).toHaveBeenCalledTimes(2)
  })

  it.each([
    { doc: 3 },
    { gid: 8 },
    { glyph: 'b' },
    { layer: 'layer-2' },
    { master: 'm2' },
  ])('does not draw a preview for another source (%j)', (change) => {
    install(translateFilter.manifest.id)
    useFilterPreview.setState({
      preview: preview({ target: { ...target, ...change } }),
    })
    const ctx = canvas()
    drawPluginOverlays(ctx, new Viewport(), { source, scene, theme })
    expect(ctx.stroke).not.toHaveBeenCalled()
  })

  it('releases old cached paths while retaining the newest previews', () => {
    install(translateFilter.manifest.id)
    const construct = vi.fn()
    vi.stubGlobal(
      'Path2D',
      class {
        constructor(data: string) {
          construct(data)
        }
      },
    )
    const ctx = canvas()
    const viewport = new Viewport()
    const paint = (index: number) => {
      useFilterPreview.setState({
        preview: preview({
          overlays: [{ kind: 'path', path: `M${1000 + index} 0L2000 1Z` }],
        }),
      })
      drawPluginOverlays(ctx, viewport, { source, scene, theme })
    }
    for (let index = 0; index < 65; index += 1) paint(index)
    paint(64)
    expect(construct).toHaveBeenCalledTimes(65)
    paint(0)
    expect(construct).toHaveBeenCalledTimes(66)
  })

  it.each([false, undefined])(
    'ignores disabled or removed preview plugins (%s)',
    (enabled) => {
      if (enabled !== undefined) install(translateFilter.manifest.id, enabled)
      useFilterPreview.setState({ preview: preview() })
      const ctx = canvas()
      drawPluginOverlays(ctx, new Viewport(), { source, scene, theme })
      expect(ctx.stroke).not.toHaveBeenCalled()
    },
  )
})
