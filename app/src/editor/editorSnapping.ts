/**
 * The editor's snapping, as one object.
 *
 * Two callers need it and they need slightly different things: a drag snaps a
 * *delta* (and must not snap to the points it is dragging), while a drawing
 * tool snaps a *point* it is about to place (and has nothing of its own to
 * avoid). Both go through here so the standing preference, the pref-driven
 * families and the match the canvas draws its halo from live in one place
 * rather than in three copies inside the controller.
 */

import type { Point2, Viewport } from '../canvas/Viewport'

import { buildSnapInput } from './editorActions'
import type { EditorLabels, EditorSource } from './editorTypes'
import type { DragGesture, SnapResult } from './gestures/drag'
import type { EditOutline } from './outlineGeometry'
import type { Selection } from './selection'
import { EMPTY_SELECTION } from './selection'
import {
  DEFAULT_SNAP_SETTINGS,
  NO_SNAP,
  snapDelta,
  type SnapMatch,
  type SnapSettings,
} from './snapping'
import type { SnappedPoint } from './tools/toolTypes'

/** Where the snap targets come from, for one call. */
export interface SnapSources {
  outline: EditOutline
  source: EditorSource | null
  selection: Selection
  drag: DragGesture | null
  labels: EditorLabels
  viewport: Viewport
}

export class SnapRunner {
  private enabled = true
  private settings: SnapSettings = DEFAULT_SNAP_SETTINGS
  private match: SnapMatch | null = null

  /** The standing preference; Ctrl suspends it for one gesture (§4.2). */
  setEnabled(enabled: boolean): void {
    this.enabled = enabled
  }

  /** Which families are live and how coarse the grid is, from the prefs. */
  setSettings(settings: SnapSettings): void {
    this.settings = settings
  }

  /** The grid step the canvas draws, or zero when there is no grid. */
  get gridSize(): number {
    return this.settings.gridSize
  }

  /** What the last snap latched onto, for the halo and the alignment lines. */
  get current(): SnapMatch | null {
    return this.match
  }

  /** Forgets the last match; a gesture that starts or ends shows no halo. */
  clear(): void {
    this.match = null
  }

  /** Snaps a drag's proposed delta. */
  forDelta(
    sources: SnapSources,
    delta: Point2,
    moved: readonly Point2[],
  ): SnapResult {
    if (!this.enabled) {
      this.match = null
      return { delta, label: null }
    }
    const match = snapDelta(
      buildSnapInput({ ...sources, settings: this.settings }),
      delta,
      moved,
    )
    this.match = match
    return { delta: match.delta, label: match.label }
  }

  /** Snaps one point a drawing tool is about to place. */
  forPoint(
    sources: SnapSources,
    point: Point2,
    suspend: boolean,
  ): SnappedPoint {
    if (suspend || !this.enabled) {
      this.match = null
      return { point, match: NO_SNAP }
    }
    const match = snapDelta(
      buildSnapInput({
        ...sources,
        // A tool is placing a *new* point: there is nothing of its own that
        // it must avoid snapping to.
        selection: EMPTY_SELECTION,
        drag: null,
        settings: this.settings,
      }),
      { x: 0, y: 0 },
      [point],
    )
    this.match = match.point ? match : null
    return {
      point: { x: point.x + match.delta.x, y: point.y + match.delta.y },
      match,
    }
  }
}
