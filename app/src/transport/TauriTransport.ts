import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type { ApiMethods, DocumentEvent } from '../api/generated'
import { ApiCallError, type Transport } from './Transport'

interface ApiErrorPayload {
  code: string
  message: string
}

function isApiErrorPayload(value: unknown): value is ApiErrorPayload {
  if (typeof value !== 'object' || value === null) return false
  const candidate = value as Partial<ApiErrorPayload>
  return (
    typeof candidate.code === 'string' && typeof candidate.message === 'string'
  )
}

export class TauriTransport implements Transport {
  readonly kind = 'tauri'

  async call<M extends keyof ApiMethods>(
    method: M,
    params: ApiMethods[M]['params'],
  ): Promise<ApiMethods[M]['result']> {
    try {
      return await invoke<ApiMethods[M]['result']>('api_call', {
        method,
        params,
      })
    } catch (error) {
      if (isApiErrorPayload(error))
        throw new ApiCallError(error.code, error.message)
      throw error
    }
  }

  onEvent(cb: (event: DocumentEvent) => void): () => void {
    let active = true
    let unlisten: UnlistenFn | undefined
    void listen<DocumentEvent>('fw:event', ({ payload }) => cb(payload)).then(
      (stopListening) => {
        if (active) unlisten = stopListening
        else stopListening()
      },
      () => undefined,
    )

    return () => {
      active = false
      unlisten?.()
      unlisten = undefined
    }
  }
}
