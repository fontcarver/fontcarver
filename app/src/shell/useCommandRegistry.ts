import { useSyncExternalStore } from 'react'

import { commandsVersion, subscribeCommands } from './commands'

/**
 * Re-renders the caller whenever the command registry changes. Components that
 * read the registry during render must call this so late registrations (start-up
 * and, later, plugins) are reflected.
 */
export function useCommandRegistry(): number {
  return useSyncExternalStore(
    subscribeCommands,
    commandsVersion,
    commandsVersion,
  )
}
