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
import { activeEditor } from '../../editor/activeEditor'
import { commit, type LayerTarget } from '../../editor/pointCommands'
import { reportError } from '../../shell/commands'

/** Applies one Info box edit and reconciles the view afterwards. */
export function commitField(
  target: LayerTarget,
  command: Command,
  label: string,
  onCommitted: () => void,
): void {
  // `doc.apply` finalises whatever coalesced edit is open, so a field commit
  // that lands inside an arrow-key run would invalidate that run's last apply.
  // The editor closes the run first, exactly as a drag does.
  void Promise.resolve(activeEditor()?.settle())
    .then(() => commit(target, command, label))
    .catch(reportError)
    .finally(onCommitted)
}
