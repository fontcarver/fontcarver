import type { ApiMethods, DocSummary, DocumentEvent, Event } from '../api'
import type { Transport } from '../transport'

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
      const wrapped: Event = 'kind' in event ? event : { kind: 'Document', ...event }
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
