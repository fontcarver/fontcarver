/**
 * The command registry. Every user-triggerable action is registered here once
 * and is then reachable from the menu bar, the command palette and a keyboard
 * shortcut without any of those three knowing about the others.
 */

/**
 * Top-level menus of the in-app menu bar. The full set from the design spec is
 * declared here from the start; menus with no commands yet render a single
 * disabled placeholder so the surface is stable and discoverable.
 */
export type MenuId =
  | 'file'
  | 'edit'
  | 'glyph'
  | 'path'
  | 'layer'
  | 'font'
  | 'features'
  | 'bitmap'
  | 'view'
  | 'window'
  | 'help'

/** Ordered list of menus rendered by the menu bar. */
export const MENU_ORDER: readonly MenuId[] = [
  'file',
  'edit',
  'glyph',
  'path',
  'layer',
  'font',
  'features',
  'bitmap',
  'view',
  'window',
  'help',
]

/** One registered command. */
export interface Command {
  /** Stable identifier, e.g. `file.open`. */
  readonly id: string
  /** i18n key of the command title. */
  readonly title: string
  /** Menu the command appears in; omitted commands are palette-only. */
  readonly menu?: MenuId
  /** Group name used to insert separators between menu sections. */
  readonly group?: string
  /** Shortcut in `Mod+Shift+P` notation. */
  readonly shortcut?: string
  /** Whether the shortcut also fires while a text field has focus. */
  readonly allowInInput?: boolean
  /** Extra text shown after the title (for example the undo label). */
  readonly detail?: () => string | undefined
  /** Enablement predicate; a command with `when() === false` cannot run. */
  readonly when?: () => boolean
  /**
   * i18n key explaining why the command is currently unavailable. Disabled
   * commands stay listed in menus and the palette, so every gated command
   * should say why.
   */
  readonly disabledReason?: string
  /** The action itself. */
  readonly run: () => void | Promise<void>
}

const registry = new Map<string, Command>()
const listeners = new Set<() => void>()
let version = 0

function notify(): void {
  version += 1
  for (const listener of [...listeners]) listener()
}

/**
 * Subscribes to registry changes. The menu bar and the palette use this so
 * commands registered after the first render (including by plugins later on)
 * appear immediately.
 */
export function subscribeCommands(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

/** Monotonic registry version, usable as a `useSyncExternalStore` snapshot. */
export function commandsVersion(): number {
  return version
}

let errorReporter: (error: unknown) => void = () => undefined

/** Installs the handler that receives errors thrown by command bodies. */
export function setCommandErrorReporter(
  reporter: (error: unknown) => void,
): void {
  errorReporter = reporter
}

/**
 * Reports an error through the same handler command bodies use.
 *
 * Anything the user triggered deserves the same feedback whether it went
 * through the command registry or straight through an editing gesture, so the
 * editor reports its transport failures here rather than inventing a channel.
 */
export function reportError(error: unknown): void {
  errorReporter(error)
}

/** Registers one command. Re-registering the same id replaces the definition. */
export function registerCommand(command: Command): void {
  registry.set(command.id, command)
  notify()
}

/** Registers several commands in order, notifying subscribers once. */
export function registerCommands(commands: readonly Command[]): void {
  for (const command of commands) registry.set(command.id, command)
  notify()
}

/** Removes a command from the registry. */
export function unregisterCommand(id: string): void {
  if (registry.delete(id)) notify()
}

/** Empties the registry. Used by tests and by hot reload. */
export function resetCommands(): void {
  registry.clear()
  notify()
}

/** Looks up one command. */
export function getCommand(id: string): Command | undefined {
  return registry.get(id)
}

/** Every registered command, in registration order. */
export function listCommands(): Command[] {
  return [...registry.values()]
}

/** Commands belonging to one menu, in registration order. */
export function commandsForMenu(menu: MenuId): Command[] {
  return listCommands().filter((command) => command.menu === menu)
}

/** Whether the command exists and its `when` predicate currently allows it. */
export function isCommandEnabled(id: string): boolean {
  const command = registry.get(id)
  if (!command) return false
  return command.when?.() ?? true
}

/**
 * Runs a command by id. Returns `false` when the command is unknown or
 * disabled. Rejections from asynchronous command bodies reach the error
 * reporter instead of becoming unhandled rejections.
 */
export function runCommand(id: string): boolean {
  const command = registry.get(id)
  if (!command || !(command.when?.() ?? true)) return false
  try {
    const result = command.run()
    if (result instanceof Promise) result.catch(errorReporter)
  } catch (error) {
    errorReporter(error)
  }
  return true
}
