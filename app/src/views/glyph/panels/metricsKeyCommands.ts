import type { Command, MetricsKeys } from '../../../api'

import { metricsKeysWith, setMetricsKeysCommand } from './panelCommands'

const EMPTY_KEYS: MetricsKeys = {
  left: null,
  right: null,
  width: null,
  vertical_origin: null,
  vertical_width: null,
}

/** Layer formulas override only the fields they explicitly supply. */
export function effectiveMetricsKeys(
  glyphKeys: MetricsKeys,
  layerKeys: MetricsKeys | null | undefined,
): MetricsKeys {
  return {
    left: layerKeys?.left ?? glyphKeys.left,
    right: layerKeys?.right ?? glyphKeys.right,
    width: layerKeys?.width ?? glyphKeys.width,
    vertical_origin: layerKeys?.vertical_origin ?? glyphKeys.vertical_origin,
    vertical_width: layerKeys?.vertical_width ?? glyphKeys.vertical_width,
  }
}

/** Show local formulas with the scope prefix, including imported formulas. */
export function displayedMetricsKey(
  glyphKeys: MetricsKeys,
  layerKeys: MetricsKeys | null | undefined,
  field: keyof MetricsKeys,
): string {
  const local = layerKeys?.[field]
  if (local == null) return glyphKeys[field] ?? ''
  if (local.startsWith('==')) return local
  return local.startsWith('=') ? `=${local}` : `==${local}`
}

/** Edit one formula's scope without changing any other formula or override. */
export function metricsKeyCommand(
  glyph: string,
  layer: string,
  glyphKeys: MetricsKeys,
  layerKeys: MetricsKeys | null | undefined,
  field: keyof MetricsKeys,
  value: string,
): Command {
  const text = value.trim()
  const local = (next: MetricsKeys): Command => ({
    SetLayerMetricsKeys: {
      glyph,
      layer,
      keys: Object.values(next).every((key) => key == null) ? null : next,
    },
  })
  if (text.startsWith('==') || (text === '' && layerKeys?.[field] != null))
    return local(metricsKeysWith(layerKeys ?? EMPTY_KEYS, field, text))

  const global = setMetricsKeysCommand(
    glyph,
    metricsKeysWith(glyphKeys, field, text),
  )
  if (layerKeys?.[field] == null) return global
  return { Batch: [global, local(metricsKeysWith(layerKeys, field, ''))] }
}

/**
 * Removes the formula a surface shows as *effective*, in every scope that
 * defines it.
 *
 * A field that edits one scope at a time — the inspector's — clears only that
 * scope, and the next one down takes over, which is what inheritance means.
 * A field that shows one effective value per metric — the spacing matrix — must
 * instead leave no formula behind at all, or the row it just set to a number
 * would immediately read as out of sync again. `null` means there was nothing
 * to clear.
 */
export function clearEffectiveMetricsKey(
  glyph: string,
  layer: string,
  glyphKeys: MetricsKeys,
  layerKeys: MetricsKeys | null | undefined,
  field: keyof MetricsKeys,
): Command | null {
  const commands: Command[] = []
  if (layerKeys?.[field] != null) {
    const next = metricsKeysWith(layerKeys, field, '')
    commands.push({
      SetLayerMetricsKeys: {
        glyph,
        layer,
        keys: Object.values(next).every((key) => key == null) ? null : next,
      },
    })
  }
  if (glyphKeys[field] != null)
    commands.push(
      setMetricsKeysCommand(glyph, metricsKeysWith(glyphKeys, field, '')),
    )
  if (commands.length === 0) return null
  return commands.length === 1 ? commands[0]! : { Batch: commands }
}

/** Strict finite decimal, matching what the kernel's float parser accepts. */
const NUMBER = /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/u

function finite(text: string): number | null {
  if (!NUMBER.test(text)) return null
  const value = Number(text)
  return Number.isFinite(value) ? value : null
}

/**
 * Splits a trailing arithmetic operation the way the kernel does: the last
 * `+ - * /` whose tail is a finite number, never at position zero.
 */
function splitOperation(body: string): {
  name: string
  divisor: number | null
} {
  for (let index = body.length - 1; index > 0; index -= 1) {
    const character = body[index]!
    if (!'+-*/'.includes(character)) continue
    const value = finite(body.slice(index + 1))
    if (value === null) continue
    return {
      name: body.slice(0, index),
      divisor: character === '/' ? value : null,
    }
  }
  return { name: body, divisor: null }
}

/**
 * Whether a reference ends in an operator with nothing usable after it.
 *
 * `-` is deliberately absent: the kernel's reference grammar allows it inside a
 * glyph name, so `=H-` names the glyph `H-` rather than being a half-typed
 * subtraction, and refusing it here would make that glyph unreachable.
 */
function danglingOperator(reference: string): boolean {
  return /[+*/]\s*$/u.test(reference) && reference.length > 1
}

/**
 * Why a metrics formula is not valid, as a `panel.metrics.keyError.*` suffix,
 * or null when the kernel would accept it.
 *
 * The text is normalised first, so the bare `n` spelling of `=n` validates as
 * the formula it stands for and no caller has to normalise twice.
 *
 * This mirrors `fc_doc::metrics::parse_key` so the field can refuse a formula
 * before a command round trip. It is a syntax check only: whether the named
 * glyph exists, whether the chain is cyclic and whether an automatic offset has
 * an auto-aligned component to offset are answered by the resolver, whose error
 * the panel already shows.
 */
export function metricsKeyError(text: string): string | null {
  const trimmed = normalizeMetricsKey(text)
  if (trimmed === '') return null
  let body = trimmed.slice(1)
  if (body.startsWith('=')) body = body.slice(1)
  body = body.trim()
  if (body === '') return 'empty'
  if (body.startsWith('+') || body.startsWith('-'))
    return finite(body) === null ? 'offset' : null
  if (finite(body) !== null) return null
  let reference = body
  const at = reference.lastIndexOf('@')
  if (at >= 0) {
    if (finite(reference.slice(at + 1)) === null) return 'height'
    reference = reference.slice(0, at)
  }
  const mirrored = reference.startsWith('|')
  if (mirrored) reference = reference.slice(1)
  const { name, divisor } = splitOperation(reference)
  if (divisor === 0) return 'divideByZero'
  if (danglingOperator(reference)) return 'operand'
  if (name === '') return mirrored ? null : 'empty'
  if (/[\s=|@*/+]/u.test(name)) return 'reference'
  return null
}

/**
 * Accepts the `n` spelling of `=n` by adding the sign the kernel requires.
 *
 * The requirement lists `n` and `=n` as the same formula, so a bare reference
 * or constant typed into the field becomes the canonical `=`-prefixed form
 * rather than being refused.
 */
export function normalizeMetricsKey(text: string): string {
  const trimmed = text.trim()
  if (trimmed === '' || trimmed.startsWith('=')) return trimmed
  return `=${trimmed}`
}
