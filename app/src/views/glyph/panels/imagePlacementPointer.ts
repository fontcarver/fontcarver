/**
 * The pointer grammar of an on-canvas source-image placement.
 *
 * The canvas asks this object first, before the outline editor sees a press, and
 * only while the Source image section has offered a placement session. A gesture
 * previews locally and commits once on release, so one drag is one undo entry.
 */

import {
  beginPlacement,
  placementCursor,
  placementHandleAt,
  updatePlacement,
  type PlacementGesture,
  type PlacementHandle,
} from '../../../canvas/sourceImagePlacement'
import type { Point2, Viewport } from '../../../canvas/Viewport'
import type { SourceImageDraw } from '../../../canvas/sourceImage'
import type { Affine } from '../../../editor/affine'

import type { ImagePlacementSession } from './imagePlacementSession'

/** A canvas pointer event, in the shape the glyph canvas already builds. */
export interface PlacementInput {
  x: number
  y: number
  button: number
  modifiers: { shift: boolean }
}

export class ImagePlacementPointer {
  private gesture: PlacementGesture | null = null
  private current: Affine | null = null
  private hovered: PlacementHandle | null = null

  /** Whether a drag is in progress. */
  get busy(): boolean {
    return this.gesture !== null
  }

  /** The handle under the pointer, for the cursor and the drawn emphasis. */
  get handle(): PlacementHandle | null {
    return this.gesture?.handle ?? this.hovered
  }

  /** The cursor this gesture or hover asks for, or `null` for none. */
  cursor(): string | null {
    return this.handle === null ? null : placementCursor(this.handle)
  }

  /** The image as it should be painted, including an in-progress gesture. */
  preview(image: SourceImageDraw | null): SourceImageDraw | null {
    if (!image || !this.current) return image
    return { ...image, transform: this.current }
  }

  /** Starts a gesture; `false` leaves the press to the outline editor. */
  down(
    viewport: Viewport,
    image: SourceImageDraw | null,
    session: ImagePlacementSession | null,
    input: PlacementInput,
  ): boolean {
    this.gesture = null
    this.current = null
    if (!image || !session || session.locked || input.button !== 0) return false
    const gesture = beginPlacement(viewport, image, { x: input.x, y: input.y })
    if (!gesture) return false
    this.gesture = gesture
    this.current = gesture.start
    return true
  }

  /** Tracks a gesture, or updates the hovered handle when idle. */
  move(
    viewport: Viewport,
    image: SourceImageDraw | null,
    input: PlacementInput,
  ): boolean {
    const at: Point2 = { x: input.x, y: input.y }
    if (!this.gesture) {
      const next = image ? placementHandleAt(viewport, image, at) : null
      const changed = next !== this.hovered
      this.hovered = next
      return changed
    }
    if (!image) return true
    this.current = updatePlacement(
      this.gesture,
      image,
      viewport.toFont(at),
      input.modifiers.shift,
    )
    return true
  }

  /** Commits a finished gesture; `false` leaves the release to the editor. */
  up(session: ImagePlacementSession | null): boolean {
    const gesture = this.gesture
    const transform = this.current
    this.gesture = null
    if (!gesture) {
      this.current = null
      return false
    }
    // A press that never moved changes nothing, so it writes nothing.
    if (
      session &&
      !session.locked &&
      transform &&
      transform.some((value, index) => value !== gesture.start[index])
    )
      session.commit(transform)
    this.current = null
    return true
  }

  /** Abandons an open gesture without writing it. */
  cancel(): void {
    this.gesture = null
    this.current = null
    this.hovered = null
  }
}
