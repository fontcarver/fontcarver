/**
 * Keyboard shortcut parsing, formatting and global binding.
 *
 * Shortcuts are written in `Mod+Shift+P` notation: `Mod` is Command on macOS
 * and Control everywhere else, so one string serves every platform.
 */

import { useUiStore } from '../stores/uiStore'
import { useKeymapStore } from '../stores/keymapStore'

import { listCommands, runCommand } from './commands'

export {
  formatShortcut,
  isMacPlatform,
  matchesShortcut,
  parseShortcut,
} from './shortcutSyntax'
export type { ParsedShortcut } from './shortcutSyntax'
import { isMacPlatform, matchesShortcut, parseShortcut } from './shortcutSyntax'
import { shortcutFor } from './keymap'
import { getCommand } from './commands'

/** Whether the event target is a text field that should swallow shortcuts. */
export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  if (
    target.isContentEditable ||
    target.closest('[contenteditable]:not([contenteditable="false"])')
  )
    return true
  const tag = target.tagName
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT'
}

/**
 * Whether the target is a natively interactive control that owns keys such as
 * Space and Enter — a button, a link with `href`, a select, a summary, an
 * element with `role="button"`, or anything made focusable with `tabindex`.
 * A `<canvas>` is excluded: the glyph view makes its canvas focusable so it can
 * own Space itself, so a focusable canvas must not read as an interactive
 * control.
 */
export function isInteractiveTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  if (target instanceof HTMLCanvasElement) return false
  const tag = target.tagName
  if (tag === 'BUTTON' || tag === 'SELECT' || tag === 'SUMMARY') return true
  if (tag === 'A' && target.hasAttribute('href')) return true
  if (target.getAttribute('role') === 'button') return true
  return target.hasAttribute('tabindex')
}

/**
 * Finds the command bound to a keyboard event, honouring `allowInInput` and
 * each command's `when` guard: a command that is currently disabled is not
 * treated as bound, so the dispatcher leaves the key to its native action.
 */
export function commandForEvent(
  event: KeyboardEvent,
  mac: boolean = isMacPlatform(),
): string | undefined {
  if (event.defaultPrevented || event.isComposing || event.keyCode === 229)
    return undefined
  const ui = useUiStore.getState()
  const textMode =
    ui.tabs.find((tab) => tab.id === ui.activeTab)?.textState?.mode === 'text'
  if (
    textMode &&
    !event.ctrlKey &&
    !event.metaKey &&
    !event.altKey &&
    ((event.key.length === 1 && event.key !== ' ') || event.key === 'Backspace')
  )
    return undefined
  const editable = isEditableTarget(event.target)
  const overrides = useKeymapStore.getState().overrides
  // A newly shipped default must not steal an existing user binding.
  const commands = listCommands().sort(
    (a, b) =>
      Number(Object.hasOwn(overrides, b.id)) -
      Number(Object.hasOwn(overrides, a.id)),
  )
  for (const command of commands) {
    const shortcut = shortcutFor(command.id, command.shortcut, overrides)
    if (!shortcut) continue
    const parsed = parseShortcut(shortcut)
    if (
      editable &&
      (!command.allowInInput ||
        (parsed &&
          !parsed.mod &&
          !parsed.control &&
          !parsed.alt &&
          !/^f\d+$/.test(parsed.key)))
    )
      continue
    if (
      parsed &&
      !parsed.mod &&
      !parsed.control &&
      !parsed.alt &&
      !/^f\d+$/.test(parsed.key) &&
      isInteractiveTarget(event.target)
    )
      continue
    if (!parsed || !matchesShortcut(event, parsed, mac)) continue
    // Honour the command's `when` guard before claiming the event. A command
    // whose `when()` is currently false is not bound to this keystroke, so the
    // dispatcher must not `preventDefault` — the key proceeds to its native
    // action (for example Space activating the focused button).
    if (command.when && !command.when()) continue
    // A customized tool retains its priority over newly shipped defaults,
    // but its tap/hold lifecycle belongs to useToolShortcuts, not runCommand.
    if (command.id.startsWith('tool.')) return undefined
    return command.id
  }
  return undefined
}

/**
 * Whether a modal dialog currently owns the keyboard.
 *
 * Escape is exempt: it is the way out of every dialog, and no dialog would ever
 * want the dispatcher to swallow it.
 */
function claimedByModal(event: KeyboardEvent, id: string): boolean {
  if (getCommand(id)?.allowInModal) return false
  if (event.key.toLowerCase() === 'escape') return false
  return useUiStore.getState().modalCount > 0
}

/**
 * Binds every registered shortcut on the given target. Returns a disposer.
 * Bindings are resolved per event, so commands registered later are honoured.
 *
 * While a modal dialog is on screen the dispatcher stands down: `Mod+W` behind
 * an open Export dialog must not close the document the dialog is exporting.
 */
export function installShortcuts(
  target: Pick<Window, 'addEventListener' | 'removeEventListener'> = window,
): () => void {
  const mac = isMacPlatform()
  const handler = (event: Event): void => {
    if (!(event instanceof KeyboardEvent) || event.repeat) return
    const id = commandForEvent(event, mac)
    if (!id || claimedByModal(event, id)) return
    event.preventDefault()
    runCommand(id)
  }
  target.addEventListener('keydown', handler, true)
  return () => target.removeEventListener('keydown', handler, true)
}
