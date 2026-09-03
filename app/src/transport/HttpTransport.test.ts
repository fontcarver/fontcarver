import { afterEach, describe, expect, it, vi } from 'vitest'

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
})
