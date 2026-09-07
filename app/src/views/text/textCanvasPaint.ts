import type { Viewport } from '../../canvas/Viewport'
import type { SurroundingGlyph, TextCanvasRun } from './textCanvas'
import { surroundingGlyphs, visibleCanvasRun } from './textCanvasBridge'

/** Keeps run culling and path parsing off the glyph editor's drag paint loop. */
export class TextCanvasPaint {
  private readonly paths = new Map<string, Path2D>()
  private run: TextCanvasRun | null = null
  private geometry = ''
  private showActive = false
  private result: { visible: number[]; surrounding: SurroundingGlyph[] } = {
    visible: [],
    surrounding: [],
  }

  prepare(run: TextCanvasRun, viewport: Viewport, showActive: boolean) {
    const geometry = [
      viewport.scale,
      viewport.offset.x,
      viewport.offset.y,
      viewport.width,
      viewport.height,
    ].join(':')
    if (
      run === this.run &&
      geometry === this.geometry &&
      showActive === this.showActive
    )
      return this.result
    this.run = run
    this.geometry = geometry
    this.showActive = showActive
    if (this.paths.size > 1024) this.paths.clear()
    const visible = visibleCanvasRun(run, viewport)
    this.result = {
      visible,
      surrounding: surroundingGlyphs(
        showActive ? { ...run, mode: 'text' } : run,
        this.paths,
        visible,
      ),
    }
    return this.result
  }
}
