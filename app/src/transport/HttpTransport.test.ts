import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { ApiCallError } from './Transport'
import { HttpTransport } from './HttpTransport'

describe('HttpTransport.call', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('returns the result from a successful API response', async () => {
    const result = [{ doc: 1, family_name: 'Test' }]
    const fetchMock = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(
        new Response(JSON.stringify({ result }), { status: 200 }),
      )
    const transport = new HttpTransport('http://localhost:8787')

    await expect(transport.call('session.list', {})).resolves.toEqual(result)
    expect(fetchMock).toHaveBeenCalledWith(
      'http://localhost:8787/api/session.list',
      {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: '{}',
        signal: expect.any(AbortSignal),
      },
    )
  })

  it('turns an invalid_params response into ApiCallError', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(
        JSON.stringify({
          error: { code: 'invalid_params', message: 'path is required' },
        }),
        { status: 400 },
      ),
    )
    const transport = new HttpTransport('http://localhost:8787')

    const error = await transport
      .call('session.open', { path: '', face_index: null })
      .catch((e) => e)

    expect(error).toBeInstanceOf(ApiCallError)
    expect(error).toMatchObject({
      code: 'invalid_params',
      message: 'path is required',
    })
  })

  it('propagates network failures', async () => {
    const failure = new TypeError('network unavailable')
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(failure)
    const transport = new HttpTransport('http://localhost:8787')

    await expect(transport.call('session.list', {})).rejects.toBe(failure)
  })

  it('turns a non-JSON HTTP error into an internal ApiCallError', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('<html>bad gateway</html>', {
        status: 502,
        statusText: 'Bad Gateway',
      }),
    )
    const transport = new HttpTransport('http://localhost:8787')

    const error = await transport.call('session.list', {}).catch((e) => e)

    expect(error).toBeInstanceOf(ApiCallError)
    expect(error).toMatchObject({ code: 'internal' })
    expect(error.message).toContain('HTTP 502')
  })

  it('times out a hung request with an io ApiCallError', async () => {
    vi.spyOn(globalThis, 'fetch').mockImplementation((_input, init) => {
      return new Promise((_resolve, reject) => {
        init?.signal?.addEventListener('abort', () => reject(init.signal?.reason))
      })
    })
    const transport = new HttpTransport('http://localhost:8787', {
      timeoutMs: 1,
    })

    await expect(transport.call('session.list', {})).rejects.toMatchObject({
      code: 'io',
    })
  })
})

class FakeWebSocket {
  static instances: FakeWebSocket[] = []

  readonly url: string
  onopen: (() => void) | null = null
  onmessage: ((event: { data: unknown }) => void) | null = null
  onclose: (() => void) | null = null
  onerror: (() => void) | null = null
  close = vi.fn(() => this.onclose?.())

  constructor(url: string | URL) {
    this.url = url.toString()
    FakeWebSocket.instances.push(this)
  }
}

describe('HttpTransport events', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    FakeWebSocket.instances = []
    vi.stubGlobal('WebSocket', FakeWebSocket)
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('backs off reconnects to five seconds and resets after opening', async () => {
    const transport = new HttpTransport('http://localhost:8787')
    const unsubscribe = transport.onEvent(vi.fn())
    const delays = [250, 500, 1_000, 2_000, 4_000, 5_000, 5_000]

    for (const [index, delay] of delays.entries()) {
      FakeWebSocket.instances[index].onclose?.()
      await vi.advanceTimersByTimeAsync(delay - 1)
      expect(FakeWebSocket.instances).toHaveLength(index + 1)
      await vi.advanceTimersByTimeAsync(1)
      expect(FakeWebSocket.instances).toHaveLength(index + 2)
    }

    FakeWebSocket.instances.at(-1)?.onopen?.()
    FakeWebSocket.instances.at(-1)?.onclose?.()
    await vi.advanceTimersByTimeAsync(249)
    expect(FakeWebSocket.instances).toHaveLength(delays.length + 1)
    await vi.advanceTimersByTimeAsync(1)
    expect(FakeWebSocket.instances).toHaveLength(delays.length + 2)
    unsubscribe()
  })

  it('closes the socket and clears reconnect timers after last unsubscribe', () => {
    const transport = new HttpTransport('http://localhost:8787')
    const unsubscribe = transport.onEvent(vi.fn())
    FakeWebSocket.instances[0].onclose?.()
    expect(vi.getTimerCount()).toBe(1)
    vi.advanceTimersByTime(250)
    const socket = FakeWebSocket.instances[1]

    unsubscribe()

    expect(socket.close).toHaveBeenCalledOnce()
    expect(vi.getTimerCount()).toBe(0)
  })

  it('keeps one socket until both listeners unsubscribe', () => {
    const transport = new HttpTransport('http://localhost:8787')
    const firstUnsubscribe = transport.onEvent(vi.fn())
    const secondUnsubscribe = transport.onEvent(vi.fn())
    const socket = FakeWebSocket.instances[0]
    expect(FakeWebSocket.instances).toHaveLength(1)

    firstUnsubscribe()
    expect(socket.close).not.toHaveBeenCalled()

    secondUnsubscribe()
    expect(socket.close).toHaveBeenCalledOnce()
    expect(vi.getTimerCount()).toBe(0)
  })
})
