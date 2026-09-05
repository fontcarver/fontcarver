import { call } from '../api'
import { useUiStore } from '../stores/uiStore'

/**
 * Asks the kernel for crash-recovery snapshots and opens the dialog when there
 * are any.
 *
 * Called once at start-up. A failure here is deliberately quiet: not being able
 * to list snapshots must not greet the user with an error before the shell has
 * even finished loading, and the same list is reachable again from the kernel.
 */
export async function checkForRecovery(): Promise<void> {
  try {
    const entries = await call('session.recover_list', {})
    if (entries.length === 0) return
    useUiStore.getState().setDialog({ kind: 'recovery', entries })
  } catch {
    /* No recovery listing: start with an empty session. */
  }
}
