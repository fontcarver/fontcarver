/**
 * Shared shape of the Info box row groups.
 *
 * Every group edits the same layer through the same command path, so they take
 * the same props; keeping the type in one place is what lets the Info box
 * switch on the selection without each group inventing its own contract.
 */

import type { LayerOutline } from '../../api'
import type { LayerTarget } from '../../editor/pointCommands'
import type { Selection } from '../../editor/selection'

/** What every Info box row group needs. */
export interface InfoRowsProps {
  target: LayerTarget
  outline: LayerOutline
  selection: Selection
  /** Master the layer belongs to, for master-scoped guides. */
  master: string | null
  /** Called after a command was applied so the view refetches. */
  onCommitted: () => void
}
