/** Portable shortcut parsing and display, independent of application state. */

/** A shortcut broken into modifier flags and a normalised key. */
export interface ParsedShortcut {
  readonly mod: boolean
  /** Physical Control, independent of the platform's primary modifier. */
  readonly control?: boolean
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
  const parts = shortcut.split('+').map((part) => part.trim())

  if (parts.length === 0 || parts.some((part) => !part)) return null
  let mod = false
  let control = false
  let shift = false
  let alt = false
  let key: string | undefined
  for (const part of parts) {
    const lower = part.toLowerCase()
    if (lower === 'mod' || lower === 'cmd' || lower === 'ctrl') {
      // Ctrl/Cmd remain portable aliases for older saved keymaps.
      if (mod || control) return null
      mod = true
    } else if (lower === 'control') {
      if (control || mod) return null
      control = true
    } else if (lower === 'shift') {
      if (shift) return null
      shift = true
    } else if (lower === 'alt' || lower === 'option') {
      if (alt) return null
      alt = true
    } else {
      if (key !== undefined) return null
      key = normaliseKey(part)
      if (
        !/^(?:[^\s]{1}|f(?:[1-9]|1[0-9]|2[0-4])|arrow(?:left|right|up|down)|escape|enter|space|delete|backspace|home|end|pageup|pagedown|tab)$/.test(
          key,
        )
      )
        return null
    }
  }
  return key === undefined
    ? null
    : { mod, ...(control ? { control: true } : {}), shift, alt, key }
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
  tab: 'Tab',
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
    (parsed.key.length === 1 || /^f\d+$/.test(parsed.key)
      ? parsed.key.toUpperCase()
      : parsed.key)
  if (mac) {
    return `${parsed.control ? '⌃' : ''}${parsed.mod ? MAC_SYMBOLS.mod : ''}${parsed.alt ? MAC_SYMBOLS.alt : ''}${
      parsed.shift ? MAC_SYMBOLS.shift : ''
    }${label}`
  }
  const parts: string[] = []
  if (parsed.mod || parsed.control) parts.push('Ctrl')
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
  const control = Boolean(parsed.control || (!mac && parsed.mod))
  const meta = mac && parsed.mod
  if (event.ctrlKey !== control || event.metaKey !== meta) return false
  if (event.shiftKey !== parsed.shift) return false
  if (event.altKey !== parsed.alt) return false
  return eventKeys(event).includes(parsed.key)
}
