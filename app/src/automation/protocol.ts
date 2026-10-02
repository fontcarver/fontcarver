import type { ApiMethods, DocId, GlyphId, MasterId } from '../api/generated'

/** Selection captured when the user explicitly starts a macro. */
export interface MacroContext {
  doc: DocId | null
  master: MasterId | null
  glyph: GlyphId | null
}

/** Macros cannot recursively launch scripts or leave gesture transactions open. */
export type MacroMethod = Exclude<
  keyof ApiMethods,
  | `automation.${string}`
  | 'doc.edit_begin'
  | 'doc.edit_apply'
  | 'doc.edit_end'
  | 'doc.edit_cancel'
>

/** Generated API contract shared by JavaScript macro facades and the bridge. */
export type MacroApiCall = <M extends MacroMethod>(
  method: M,
  params: ApiMethods[M]['params'],
) => Promise<ApiMethods[M]['result']>

/** Available as both `fc` and `fontcarver` inside a JavaScript macro. */
export interface MacroFacade {
  readonly context: Readonly<MacroContext>
  readonly call: MacroApiCall
}

export type MacroLevel = 'log' | 'info' | 'warn' | 'error' | 'result'
export interface MacroOutput {
  level: MacroLevel
  text: string
}

export const MACRO_LIMITS = Object.freeze({
  sourceBytes: 64 * 1024,
  messageBytes: 1024 * 1024,
  calls: 256,
  outputLines: 256,
  outputBytes: 64 * 1024,
  timeoutMs: 10_000,
  maxTimeoutMs: 30_000,
})

export type WorkerRequest =
  | {
      type: 'call'
      id: number
      method: MacroMethod
      params: Record<string, unknown>
    }
  | ({ type: 'output' } & MacroOutput)
  | { type: 'done' }
  | { type: 'failure'; error: string }

export type WorkerResponse =
  | { type: 'start'; source: string; context: MacroContext }
  | { type: 'response'; id: number; result?: unknown; error?: string }

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function byteLength(value: string): number {
  return new TextEncoder().encode(value).byteLength
}

export function withinMessageLimit(value: unknown): boolean {
  try {
    const json = JSON.stringify(value)
    return (
      typeof json === 'string' && byteLength(json) <= MACRO_LIMITS.messageBytes
    )
  } catch {
    return false
  }
}

export function isMacroMethod(value: unknown): value is MacroMethod {
  // The generated request dispatcher remains the authority for method names
  // and parameter schemas. Never forward recursive process/runtime methods.
  return (
    typeof value === 'string' &&
    value.length <= 128 &&
    /^[a-z][a-z0-9_]*\.[a-z][a-z0-9_]*$/.test(value) &&
    !value.startsWith('automation.') &&
    ![
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
      'doc.edit_cancel',
    ].includes(value)
  )
}

export function isContext(value: unknown): value is MacroContext {
  if (!isRecord(value)) return false
  const id = (item: unknown) =>
    item === null ||
    (typeof item === 'number' && Number.isSafeInteger(item) && item >= 0)
  return (
    id(value.doc) &&
    id(value.glyph) &&
    (value.master === null || typeof value.master === 'string')
  )
}

export function isWorkerRequest(value: unknown): value is WorkerRequest {
  if (!isRecord(value) || !withinMessageLimit(value)) return false
  switch (value.type) {
    case 'call':
      return (
        Number.isSafeInteger(value.id) &&
        (value.id as number) > 0 &&
        isMacroMethod(value.method) &&
        isRecord(value.params)
      )
    case 'output':
      return (
        typeof value.level === 'string' &&
        ['log', 'info', 'warn', 'error', 'result'].includes(value.level) &&
        typeof value.text === 'string'
      )
    case 'done':
      return true
    case 'failure':
      return typeof value.error === 'string'
    default:
      return false
  }
}

export function errorText(error: unknown): string {
  return (error instanceof Error ? error.message : String(error)).slice(0, 4096)
}
