/**
 * Shared shape of the Info box row groups.
 *
 * Every group edits the same layer through the same command path, so they take
 * the same props; keeping the type in one place is what lets the Info box
 * switch on the selection without each group inventing its own contract.
 */

import type {
  Anchor,
  ComponentView,
  Contour,
  Guide,
  LayerOutline,
} from '../../api'
import type { LayerTarget } from '../../editor/pointCommands'
import type { Selection } from '../../editor/selection'

/**
 * A layer as the Info box reads it.
 *
 * The geometry is readonly because during a gesture it is the canvas's live
 * preview rather than the fetched outline, and nothing in the box mutates it.
 */
export type ReadableOutline = Omit<
  LayerOutline,
  'contours' | 'anchors' | 'components'
> & {
  contours: readonly Contour[]
  anchors: readonly Anchor[]
  components: readonly ComponentView[]
}

/** What every Info box row group needs. */
export interface InfoRowsProps {
  target: LayerTarget
  outline: ReadableOutline
  selection: Selection
  /** Master the layer belongs to, for master-scoped guides. */
  master: string | null
  /** The master's own guides, which the canvas can select just as well. */
  masterGuides: readonly Guide[]
  /** Called after a command was applied so the view refetches. */
  onCommitted: () => void
}
