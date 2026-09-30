/** Geometry and source identity owned by one editor controller. */

import type { Anchor, ComponentView, Contour } from '../api'
import type { Point2, Viewport } from '../canvas/Viewport'

import type { EditorPreview, EditorSource } from './editorTypes'
import { hitTest, type Hit, type HitTestInput } from './hitTest'
import { buildEditOutline, type EditOutline } from './outlineGeometry'
import type { LayerTarget } from './pointCommands'
import type { EditTarget, Selection } from './selection'
import {
  editableContour,
  editableOutline,
  editorMetadata,
  inPathFocus,
} from './editorMetadata'

/**
 * Keeps the authoritative source and optimistic geometry in one index.
 * Rebuilding the edit outline is memoized by contour-array identity because
 * snapping and painting ask for it more than once per pointer move.
 */
export class EditorGeometry {
  source: EditorSource | null = null
  private pathFocus: ReadonlySet<number> | null = null
  /** CSS-pixel node hit radius; other families keep their relative sizes. */
  hitDistance = 5
  /** Drawn on-curve marker diameter in CSS pixels, from the preferences. */
  nodeSize = 5
  /** Drawn off-curve marker diameter in CSS pixels, from the preferences. */
  handleSize = 3.5
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
    if (changed || source === null) this.pathFocus = null
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

  /** Limit hit testing and selection to a session-local set of source paths. */
  setPathFocus(focus: ReadonlySet<number> | null): void {
    this.pathFocus = focus
  }

  /** Preserve stored indexes while excluding locked and unfocused paths. */
  selectableOutline(): EditOutline {
    return editableOutline(
      this.editOutline(),
      editorMetadata(this.source?.editorMetadata).lockedPaths,
      this.pathFocus,
      this.source?.glyphLocked ?? false,
    )
  }

  /** Eligibility shared by clicks, marquees, lasso and keyboard selection. */
  canSelect(target: EditTarget): boolean {
    if (this.source?.glyphLocked || !inPathFocus(target, this.pathFocus))
      return false
    if ('contour' in target)
      return editableContour(
        this.contours(),
        target.contour,
        editorMetadata(this.source?.editorMetadata).lockedPaths,
        this.pathFocus,
      )
    if (target.kind === 'component')
      return !this.components()[target.index]?.component.locked
    if (target.kind === 'guide')
      return !this.source?.guides.find(
        (entry) => entry.scope === target.scope && entry.index === target.index,
      )?.guide.locked
    return true
  }

  layerTarget(): LayerTarget | null {
    const source = this.source
    return source
      ? { doc: source.doc, glyph: source.glyph, layer: source.outline.layer }
      : null
  }

  hitInput(viewport: Viewport, selection: Selection): HitTestInput {
    return {
      outline: this.selectableOutline(),
      anchors: this.source?.glyphLocked || this.pathFocus ? [] : this.anchors(),
      guides:
        this.source?.glyphLocked || this.pathFocus
          ? []
          : (this.source?.guides ?? []).filter((entry) => !entry.guide.locked),
      components:
        this.source?.glyphLocked || this.pathFocus
          ? []
          : this.components().map((entry) =>
              entry.component.locked ? { ...entry, bbox: null } : entry,
            ),
      viewport,
      selection,
      hitDistance: this.hitDistance,
      nodeSize: this.nodeSize,
      handleSize: this.handleSize,
    }
  }

  hitAt(viewport: Viewport, selection: Selection, at: Point2): Hit | null {
    if (!this.source) return null
    const hit = hitTest(this.hitInput(viewport, selection), at)
    return hit && this.canSelect(hit.target) ? hit : null
  }

  /**
   * What a press landed on when nothing selectable was there.
   *
   * `hitInput` has already taken the locked paths, components and guides out,
   * so a press on one is indistinguishable from a press on empty canvas — which
   * is exactly the silence a user cannot act on. This second, unfiltered hit
   * test answers "was there something there that you are not allowed to move?"
   * so the canvas can say so.
   */
  refusedAt(viewport: Viewport, selection: Selection, at: Point2): boolean {
    const source = this.source
    if (!source) return false
    const hit = hitTest(
      {
        outline: this.editOutline(),
        anchors: this.anchors(),
        guides: source.guides,
        components: this.components(),
        viewport,
        selection,
        hitDistance: this.hitDistance,
        nodeSize: this.nodeSize,
        handleSize: this.handleSize,
      },
      at,
    )
    return hit !== null && !this.canSelect(hit.target)
  }
}
