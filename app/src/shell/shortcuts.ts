/**
 * Keyboard shortcut parsing, formatting and global binding.
 *
 * Shortcuts are written in `Mod+Shift+P` notation: `Mod` is Command on macOS
 * and Control everywhere else, so one string serves every platform.
 */

import { listCommands, runCommand } from './commands'

/** A shortcut broken into modifier flags and a normalised key. */
export interface ParsedShortcut {
  readonly mod: boolean
  readonly shift: boolean
  readonly alt: boolean
  readonly key: string
}

/** Whether the current platform uses Command as the primary modifier. */
export function isMacPlatform(): boolean {
  const nav = globalThis.navigator as
    (Navigator & { userAgentData?: { platform?: string } }) | undefined
  const platform = nav?.userAgentData?.platform ?? nav?.platform ?? ''
  return /mac|iphone|ipad|ipod/i.test(platform)
}

const CODE_KEYS: Record<string, string> = {
  Equal: '=',
  NumpadAdd: '=',
  Minus: '-',
  NumpadSubtract: '-',
  Slash: '/',
  Comma: ',',
  Period: '.',
  BracketLeft: '[',
  BracketRight: ']',
}

function normaliseKey(key: string): string {
  const lower = key.toLowerCase()
  if (lower === '+' || lower === 'add') return '='
  if (lower === '_' || lower === 'subtract') return '-'
  if (lower === 'esc') return 'escape'
  if (lower === ' ') return 'space'
  return lower
}

/** Parses `Mod+Shift+P`; returns `null` for an empty or malformed shortcut. */
export function parseShortcut(shortcut: string): ParsedShortcut | null {
  const parts = shortcut
    .split('+')
    .map((part) => part.trim())
    .filter((part) => part.length > 0)
  if (parts.length === 0) return null
  let mod = false
  let shift = false
  let alt = false
  let key: string | undefined
  for (const part of parts) {
    const lower = part.toLowerCase()
    if (lower === 'mod' || lower === 'cmd' || lower === 'ctrl') mod = true
    else if (lower === 'shift') shift = true
    else if (lower === 'alt' || lower === 'option') alt = true
    else key = normaliseKey(part)
  }
  return key === undefined ? null : { mod, shift, alt, key }
}

const MAC_SYMBOLS: Record<string, string> = {
  mod: '⌘',
  shift: '⇧',
  alt: '⌥',
}

const KEY_LABELS: Record<string, string> = {
  arrowleft: '←',
  arrowright: '→',
  arrowup: '↑',
  arrowdown: '↓',
  escape: 'Esc',
  enter: '↵',
  space: 'Space',
}

/** Renders a shortcut for display: `⌘⇧P` on macOS, `Ctrl+Shift+P` elsewhere. */
export function formatShortcut(
  shortcut: string,
  mac: boolean = isMacPlatform(),
): string {
  const parsed = parseShortcut(shortcut)
  if (!parsed) return ''
  const label =
    KEY_LABELS[parsed.key] ??
    (parsed.key.length === 1 ? parsed.key.toUpperCase() : parsed.key)
  if (mac) {
    return `${parsed.mod ? MAC_SYMBOLS.mod : ''}${parsed.alt ? MAC_SYMBOLS.alt : ''}${
      parsed.shift ? MAC_SYMBOLS.shift : ''
    }${label}`
  }
  const parts: string[] = []
  if (parsed.mod) parts.push('Ctrl')
  if (parsed.alt) parts.push('Alt')
  if (parsed.shift) parts.push('Shift')
  parts.push(label)
  return parts.join('+')
}

function eventKeys(event: KeyboardEvent): string[] {
  const keys = [normaliseKey(event.key)]
  const code = event.code
  if (code in CODE_KEYS) keys.push(CODE_KEYS[code] ?? '')
  else if (code.startsWith('Key')) keys.push(code.slice(3).toLowerCase())
  else if (code.startsWith('Digit')) keys.push(code.slice(5))
  else if (code.startsWith('Numpad')) keys.push(code.slice(6).toLowerCase())
  return keys.filter((key) => key.length > 0)
}

/** Whether a keyboard event matches a parsed shortcut on this platform. */
export function matchesShortcut(
  event: KeyboardEvent,
  parsed: ParsedShortcut,
  mac: boolean = isMacPlatform(),
): boolean {
  const modPressed = mac ? event.metaKey : event.ctrlKey
  const otherMod = mac ? event.ctrlKey : event.metaKey
  if (modPressed !== parsed.mod || otherMod) return false
  if (event.shiftKey !== parsed.shift) return false
  if (event.altKey !== parsed.alt) return false
  return eventKeys(event).includes(parsed.key)
}

/** Whether the event target is a text field that should swallow shortcuts. */
export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  if (target.isContentEditable) return true
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
  const editable = isEditableTarget(event.target)
  for (const command of listCommands()) {
    if (!command.shortcut) continue
    if (editable && !command.allowInInput) continue
    const parsed = parseShortcut(command.shortcut)
    if (!parsed || !matchesShortcut(event, parsed, mac)) continue
    // Honour the command's `when` guard before claiming the event. A command
    // whose `when()` is currently false is not bound to this keystroke, so the
    // dispatcher must not `preventDefault` — the key proceeds to its native
    // action (for example Space activating the focused button).
    if (command.when && !command.when()) continue
    return command.id
  }
  return undefined
}

/**
 * Binds every registered shortcut on the given target. Returns a disposer.
 * Bindings are resolved per event, so commands registered later are honoured.
 */
export function installShortcuts(
  target: Pick<Window, 'addEventListener' | 'removeEventListener'> = window,
): () => void {
  const mac = isMacPlatform()
  const handler = (event: Event): void => {
    if (!(event instanceof KeyboardEvent) || event.repeat) return
    const id = commandForEvent(event, mac)
    if (!id) return
    event.preventDefault()
    runCommand(id)
  }
  target.addEventListener('keydown', handler, true)
  return () => target.removeEventListener('keydown', handler, true)
}
