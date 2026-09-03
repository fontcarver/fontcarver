import type { ApiMethods, DocumentEvent } from '../api/generated'

export interface Transport {
  readonly kind: 'tauri' | 'http'
  call<M extends keyof ApiMethods>(
    method: M,
    params: ApiMethods[M]['params'],
  ): Promise<ApiMethods[M]['result']>
  onEvent(cb: (event: DocumentEvent) => void): () => void
}

export class ApiCallError extends Error {
  readonly code: string

  constructor(code: string, message: string) {
    super(message)
    this.name = 'ApiCallError'
    this.code = code
  }
}
