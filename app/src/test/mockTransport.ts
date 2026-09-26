import type {
  ApiMethods,
  Change,
  DocSummary,
  DocumentEvent,
  Event,
} from '../api'
import { ApiCallError, type Transport } from '../transport'

/** One recorded API call. */
export interface RecordedCall {
  method: keyof ApiMethods
  params: unknown
}

/** Per-method fake implementations; unlisted methods reject. */
export type MockHandlers = {
  [M in keyof ApiMethods]?: (
    params: ApiMethods[M]['params'],
  ) => ApiMethods[M]['result'] | Promise<ApiMethods[M]['result']>
}

/** A transport double that records calls and can push events. */
export interface MockTransport extends Transport {
  readonly calls: RecordedCall[]
  emit: (event: DocumentEvent | Event) => void
}

/** Creates a transport double for store and component tests. */
export function createMockTransport(
  handlers: MockHandlers = {},
): MockTransport {
  const calls: RecordedCall[] = []
  const listeners = new Set<(event: Event) => void>()

  return {
    kind: 'http',
    calls,
    async call(method, params) {
      calls.push({ method, params })
      const handler = handlers[method] as
        ((value: unknown) => unknown) | undefined
      if (!handler) throw new Error(`unhandled method: ${method}`)
      return (await handler(params)) as never
    },
    onEvent(cb) {
      listeners.add(cb)
      return () => listeners.delete(cb)
    },
    emit(event) {
      const wrapped: Event =
        'kind' in event ? event : { kind: 'Document', ...event }
      for (const listener of [...listeners]) listener(wrapped)
    },
  }
}

/** A minimal document summary for tests. */
export function makeSummary(overrides: Partial<DocSummary> = {}): DocSummary {
  return {
    doc: 1,
    path: '/fonts/DejaVuSans.ttf',
    dirty: false,
    family_name: 'DejaVu Sans',
    style_name: 'Book',
    units_per_em: 2048,
    glyph_count: 6253,
    masters: [{ id: 'm1', name: 'Regular' }],
    axes: [],
    undo_label: null,
    redo_label: null,
    format: 'TrueType',
    warnings: [],
    ...overrides,
  }
}

/** An empty `Change`, which most editing calls answer with. */
export function noChange(glyphs: number[] = []): Change {
  return {
    glyphs,
    font_info: false,
    masters: false,
    layout: false,
    bitmaps: false,
    math: false,
    color: false,
    feature_source: false,
    glyph_order: false,
    metrics: false,
    layers: false,
    guides: false,
  }
}

/** Extra behaviour for the editing transport. */
export interface EditingTransportOptions {
  /** Held open until resolved; lets a test observe an in-flight call. */
  hold?: { on: 'doc.edit_apply' | 'doc.edit_end'; until: Promise<void> }
  /** Makes `doc.edit_begin` reject, to exercise the failure paths. */
  failBegin?: () => Error
}

/**
 * A transport that models the kernel's edit-token rules.
 *
 * `Document::begin_edit` finalises whatever edit is open, and an
 * `apply_in_edit` against a token that is no longer active fails with
 * `InvalidEditToken` (`crates/fc-doc/src/document.rs`). A mock that answers
 * every call cheerfully cannot see a gesture jumping the queue, which is
 * exactly the ordering these tests are about — so this one keeps the active
 * token and refuses a stale one the way the kernel does.
 */
export function createEditingTransport(
  options: EditingTransportOptions = {},
): MockTransport {
  let nextToken = 101
  let active: number | null = null

  const invalidToken = () =>
    new ApiCallError('invalid_params', 'InvalidEditToken')

  const hold = async (on: 'doc.edit_apply' | 'doc.edit_end') => {
    if (options.hold?.on === on) await options.hold.until
  }

  return createMockTransport({
    'doc.edit_begin': () => {
      if (options.failBegin) throw options.failBegin()
      // Beginning an edit finalises the one that was open.
      active = nextToken
      nextToken += 1
      return active
    },
    'doc.edit_apply': async (params) => {
      await hold('doc.edit_apply')
      if (params.token !== active) throw invalidToken()
      return noChange([42])
    },
    'doc.edit_end': async (params) => {
      await hold('doc.edit_end')
      // Ending a stale token is a silent no-op in the kernel.
      if (params.token === active) active = null
      return null
    },
    // A single-shot command finalises the active edit too.
    'doc.apply': () => {
      active = null
      return noChange([42])
    },
    // `Document::undo` finalises the active edit before it walks the stack.
    'doc.undo': () => {
      active = null
      return noChange([42])
    },
    'doc.redo': () => noChange([42]),
  })
}
