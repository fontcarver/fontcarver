import type { ApiMethods, DocumentEvent } from '../api/generated'
import { ApiCallError, type Transport } from './Transport'

interface ApiErrorPayload {
  code: string
  message: string
}

interface ErrorEnvelope {
  error?: ApiErrorPayload
}

interface ResultEnvelope<Result> {
  result: Result
}

const INITIAL_RECONNECT_DELAY_MS = 250
const MAX_RECONNECT_DELAY_MS = 5_000
const DEFAULT_TIMEOUT_MS = 30_000

export interface HttpTransportOptions {
  timeoutMs?: number
}

export class HttpTransport implements Transport {
  readonly kind = 'http'
  private readonly baseUrl: string
  private readonly timeoutMs: number
  private readonly eventListeners = new Set<(event: DocumentEvent) => void>()
  private socket: WebSocket | undefined
  private reconnectTimer: ReturnType<typeof setTimeout> | undefined
  private reconnectDelayMs = INITIAL_RECONNECT_DELAY_MS

  constructor(baseUrl: string, options: HttpTransportOptions = {}) {
    this.baseUrl = baseUrl.replace(/\/$/, '')
    this.timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS
  }

  async call<M extends keyof ApiMethods>(
    method: M,
    params: ApiMethods[M]['params'],
  ): Promise<ApiMethods[M]['result']> {
    const signal = AbortSignal.timeout(this.timeoutMs)
    let response: Response
    try {
      response = await fetch(`${this.baseUrl}/api/${method}`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(params),
        signal,
      })
    } catch (error) {
      if (signal.aborted) {
        throw new ApiCallError(
          'io',
          `API request timed out after ${this.timeoutMs} ms`,
        )
      }
      throw error
    }

    if (!response.ok) {
      let envelope: ErrorEnvelope
      try {
        envelope = (await response.json()) as ErrorEnvelope
      } catch {
        throw new ApiCallError(
          'internal',
          `HTTP ${response.status} ${response.statusText}: response was not valid JSON`,
        )
      }
      const error = 'error' in envelope ? envelope.error : undefined
      throw new ApiCallError(
        error?.code ?? 'internal',
        error?.message ??
          `HTTP ${response.status} ${response.statusText}: response did not contain an API error`,
      )
    }

    let envelope: ErrorEnvelope | ResultEnvelope<ApiMethods[M]['result']>
    try {
      envelope = (await response.json()) as
        | ErrorEnvelope
        | ResultEnvelope<ApiMethods[M]['result']>
    } catch {
      throw new ApiCallError(
        'internal',
        `HTTP ${response.status} ${response.statusText}: response was not valid JSON`,
      )
    }
    if (!('result' in envelope)) {
      throw new ApiCallError(
        'invalid_response',
        'API response did not contain a result',
      )
    }
    return envelope.result
  }

  onEvent(cb: (event: DocumentEvent) => void): () => void {
    this.eventListeners.add(cb)
    this.connectEvents()
    let active = true

    return () => {
      if (!active) return
      active = false
      this.eventListeners.delete(cb)
      if (this.eventListeners.size === 0) this.disconnectEvents()
    }
  }

  private connectEvents(): void {
    if (this.eventListeners.size === 0 || this.socket !== undefined) return
    if (this.reconnectTimer !== undefined) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = undefined
    }

    const socket = new WebSocket(this.eventsUrl())
    this.socket = socket
    socket.onopen = () => {
      this.reconnectDelayMs = INITIAL_RECONNECT_DELAY_MS
    }
    socket.onmessage = ({ data }) => {
      if (typeof data !== 'string') return
      let event: DocumentEvent
      try {
        event = JSON.parse(data) as DocumentEvent
      } catch {
        return
      }
      for (const listener of [...this.eventListeners]) listener(event)
    }
    socket.onclose = () => {
      if (this.socket === socket) this.socket = undefined
      this.scheduleReconnect()
    }
    socket.onerror = () => socket.close()
  }

  private scheduleReconnect(): void {
    if (this.eventListeners.size === 0 || this.reconnectTimer !== undefined)
      return
    const delay = this.reconnectDelayMs
    this.reconnectDelayMs = Math.min(
      this.reconnectDelayMs * 2,
      MAX_RECONNECT_DELAY_MS,
    )
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = undefined
      this.connectEvents()
    }, delay)
  }

  private disconnectEvents(): void {
    if (this.reconnectTimer !== undefined) clearTimeout(this.reconnectTimer)
    this.reconnectTimer = undefined
    const socket = this.socket
    this.socket = undefined
    socket?.close()
    this.reconnectDelayMs = INITIAL_RECONNECT_DELAY_MS
  }

  private eventsUrl(): string {
    const url = new URL(`${this.baseUrl}/events`)
    url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
    return url.toString()
  }
}
