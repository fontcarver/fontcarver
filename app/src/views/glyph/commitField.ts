/**
 * Committing one Info box field.
 *
 * Every row commits the same way — one command, one undo entry, then a refetch
 * — and every row has to survive the kernel refusing it. A failure is reported
 * through the shell's error reporter (the same channel a failed command uses,
 * which the app turns into a toast) and the view still reconciles, so the field
 * shows what the document actually holds rather than what was typed.
 */

import type { Command } from '../../api'
import { commit, type LayerTarget } from '../../editor/pointCommands'
import { reportError } from '../../shell/commands'

/** Applies one Info box edit and reconciles the view afterwards. */
export function commitField(
  target: LayerTarget,
  command: Command,
  label: string,
  onCommitted: () => void,
): void {
  void commit(target, command, label).catch(reportError).finally(onCommitted)
}
