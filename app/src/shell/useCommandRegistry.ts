import { useMemo, useSyncExternalStore } from 'react'

import {
  commandsSnapshot,
  commandsVersion,
  subscribeCommands,
  type Command,
} from './commands'

/**
 * Re-renders the caller whenever the command registry changes.
 *
 * Prefer {@link useCommandList} or {@link useCommandLookup}: a component that
 * reads the registry while rendering must read it through a value, because a
 * bare call during render is invisible to React's memoisation and to the
 * compiler's, which then keeps a late registration off the screen.
 */
export function useCommandRegistry(): number {
  return useSyncExternalStore(
    subscribeCommands,
    commandsVersion,
    commandsVersion,
  )
}

/** The registered commands, re-read whenever a registration changes. */
export function useCommandList(): readonly Command[] {
  return useSyncExternalStore(
    subscribeCommands,
    commandsSnapshot,
    commandsSnapshot,
  )
}

/** Looks commands up by id, following registrations. */
export function useCommandLookup(): (id: string) => Command | undefined {
  const commands = useCommandList()
  return useMemo(() => {
    const byId = new Map(commands.map((command) => [command.id, command]))
    return (id: string) => byId.get(id)
  }, [commands])
}
