/** Geometry and source identity owned by one editor controller. */

import type { Anchor, ComponentView, Contour } from '../api'
import type { Point2, Viewport } from '../canvas/Viewport'

import type { EditorPreview, EditorSource } from './editorTypes'
import { hitTest, type Hit, type HitTestInput } from './hitTest'
import { buildEditOutline, type EditOutline } from './outlineGeometry'
import type { LayerTarget } from './pointCommands'
import type { Selection } from './selection'

/**
 * Keeps the authoritative source and optimistic geometry in one index.
 * Rebuilding the edit outline is memoized by contour-array identity because
 * snapping and painting ask for it more than once per pointer move.
 */
export class EditorGeometry {
  source: EditorSource | null = null
  preview: EditorPreview | null = null
  private edit: EditOutline = buildEditOutline([])
  private editBase: readonly Contour[] = []
  private readonly editCache = new WeakMap<readonly Contour[], EditOutline>()

  /** Installs kernel data and reports whether its selectable source changed. */
  install(source: EditorSource | null): boolean {
    const changed =
      source !== null &&
      (source.doc !== this.source?.doc ||
        source.gid !== this.source?.gid ||
        source.outline.layer !== this.source?.outline.layer)
    this.source = source
    this.preview = null
    this.edit = buildEditOutline(source?.outline.contours ?? [])
    this.editBase = source?.outline.contours ?? []
    return changed
  }

  setPreview(preview: EditorPreview | null): void {
    this.preview = preview
  }

  contours(): readonly Contour[] {
    return this.preview?.contours ?? this.source?.outline.contours ?? []
  }

  components(): readonly ComponentView[] {
    return this.preview?.components ?? this.source?.outline.components ?? []
  }

  anchors(): readonly Anchor[] {
    return this.preview?.anchors ?? this.source?.outline.anchors ?? []
  }

  editOutline(): EditOutline {
    const contours = this.contours()
    if (contours === this.editBase) return this.edit
    const cached = this.editCache.get(contours)
    const outline = cached ?? buildEditOutline(contours)
    if (!cached) this.editCache.set(contours, outline)
    this.editBase = contours
    this.edit = outline
    return outline
  }

  layerTarget(): LayerTarget | null {
    const source = this.source
    return source
      ? { doc: source.doc, glyph: source.glyph, layer: source.outline.layer }
      : null
  }

  hitInput(viewport: Viewport, selection: Selection): HitTestInput {
    return {
      outline: this.editOutline(),
      anchors: this.anchors(),
      guides: this.source?.guides ?? [],
      components: this.components(),
      viewport,
      selection,
    }
  }

  hitAt(viewport: Viewport, selection: Selection, at: Point2): Hit | null {
    return this.source ? hitTest(this.hitInput(viewport, selection), at) : null
  }
}
