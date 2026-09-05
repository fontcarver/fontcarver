/**
 * The contracts between the editor and the view that hosts it.
 *
 * They live apart from the controller because both sides depend on them: the
 * view builds the labels and the callbacks, the controller consumes them, and
 * neither should have to import the other to say what it needs.
 */

import type {
  Anchor,
  ComponentView,
  Contour,
  DocId,
  GlyphId,
  LayerOutline,
} from '../api'
import type { SceneMetrics } from '../views/glyph/glyphScene'

import type { Modifiers } from './gestures/drag'
import type { ScopedGuide } from './hitTest'
import type { HudLabels } from './hud'
import type { Selection } from './selection'
import type { SnapNames } from './snapping'

/** The geometry a live gesture is showing, before the kernel has answered. */
export interface EditorPreview {
  contours: readonly Contour[]
  anchors: readonly Anchor[]
  components: readonly ComponentView[]
}

/** Undo labels and readout words, translated by the view. */
export interface EditorLabels {
  movePoints: (count: number) => string
  moveHandles: (count: number) => string
  moveHandle: string
  reshapeSegment: string
  moveAnchor: string
  moveComponent: string
  nudgePoints: (count: number) => string
  toggleSmooth: string
  deletePoints: string
  insertPoint: string
  convertSegment: string
  setMetrics: string
  addGuide: string
  /** The HUD line a sidebearing drag shows: the handle's name and its value. */
  metricHud: (handle: 'lsb' | 'rsb' | 'width', value: number) => string
  /** Names the HUD gives a snap target, already translated. */
  snap: SnapNames
  hud: HudLabels
}

/** The glyph and layer being edited. */
export interface EditorSource {
  doc: DocId
  gid: GlyphId
  glyph: string
  outline: LayerOutline
  guides: readonly ScopedGuide[]
  metrics: SceneMetrics
  master: string | null
}

/** What the controller reports back to React. */
export interface EditorCallbacks {
  repaint: () => void
  onSelectionChange: (selection: Selection) => void
  onEditingChange: (editing: boolean) => void
  onZoomChange: (scale: number) => void
  /** A gesture finished: refetch the outline and reconcile. */
  onReload: () => void
  onCursorChange: (cursor: string) => void
  /** The geometry a live gesture is showing, for the panels that read numbers. */
  onPreview: (preview: EditorPreview | null) => void

  /**
   * A kernel call failed. Editing must not swallow transport errors: the shell
   * turns these into a toast, the same way a failed command does.
   */
  onError: (error: unknown) => void
}

/** A pointer event, normalised to canvas space. */
export interface PointerInput {
  x: number
  y: number
  button: number
  modifiers: Modifiers
}

/** Callbacks used until the view installs its own. */
export const SILENT_CALLBACKS: EditorCallbacks = {
  repaint: () => undefined,
  onSelectionChange: () => undefined,
  onEditingChange: () => undefined,
  onZoomChange: () => undefined,
  onReload: () => undefined,
  onCursorChange: () => undefined,
  onPreview: () => undefined,
  onError: () => undefined,
}
