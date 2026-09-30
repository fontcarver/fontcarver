import { afterEach, describe, expect, it, vi } from 'vitest'

import { runJavaScript, type MacroApiCall, type MacroWorker } from './runtime'
import { installMacroWorker, type MacroScript } from './macro.worker'
import { MACRO_LIMITS, type MacroContext } from './protocol'

const context: MacroContext = { doc: 3, master: 'regular', glyph: 4 }
const call = vi.fn(async () => []) as unknown as MacroApiCall

class FakeWorker implements MacroWorker {
  onmessage: ((event: MessageEvent<unknown>) => void) | null = null
  onerror: ((event: ErrorEvent) => void) | null = null
  onmessageerror: (() => void) | null = null
  postMessage = vi.fn()
  terminate = vi.fn()

  emit(data: unknown) {
    this.onmessage?.({ data } as MessageEvent<unknown>)
  }
}

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => (resolve = done))
  return { promise, resolve }
}

afterEach(() => {
  vi.useRealTimers()
  vi.clearAllMocks()
})

describe('JavaScript macro runtime', () => {
  it('passes context and source into a worker and emits console/result output', async () => {
    const worker = new FakeWorker()
    const output = vi.fn()
    const run = runJavaScript('return 42', context, {
      call,
      onOutput: output,
      workerFactory: () => worker,
    })
    expect(worker.postMessage).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'start', source: 'return 42', context }),
    )
    worker.emit({ type: 'output', level: 'log', text: 'hello' })
    worker.emit({ type: 'output', level: 'result', text: '42' })
    worker.emit({ type: 'done' })
    await expect(run.done).resolves.toEqual({ status: 'completed' })
    expect(output.mock.calls).toEqual([
      [{ level: 'log', text: 'hello' }],
      [{ level: 'result', text: '42' }],
    ])
    expect(worker.terminate).toHaveBeenCalledOnce()
  })

  it('terminates a busy worker and waits for a dispatched mutation while discarding queued calls', async () => {
    const worker = new FakeWorker()
    const pending = deferred<unknown[]>()
    const bridge = vi.fn(() => pending.promise)
    const run = runJavaScript('while (true) {}', context, {
      call: bridge as MacroApiCall,
      workerFactory: () => worker,
    })
    worker.emit({ type: 'call', id: 1, method: 'session.list', params: {} })
    worker.emit({ type: 'call', id: 2, method: 'session.list', params: {} })
    run.stop()
    expect(worker.terminate).toHaveBeenCalledOnce()
    const settled = vi.fn()
    void run.done.then(settled)
    await Promise.resolve()
    expect(settled).not.toHaveBeenCalled()
    pending.resolve([])
    await expect(run.done).resolves.toEqual({ status: 'stopped' })
    expect(bridge).toHaveBeenCalledTimes(1)
    expect(worker.postMessage).toHaveBeenCalledTimes(1)
    run.stop()
    expect(worker.terminate).toHaveBeenCalledOnce()
  })

  it('times out an unresponsive worker without needing a worker message', async () => {
    vi.useFakeTimers()
    const worker = new FakeWorker()
    const run = runJavaScript('while (true) {}', context, {
      call,
      workerFactory: () => worker,
      timeoutMs: 10,
    })
    await vi.advanceTimersByTimeAsync(10)
    await expect(run.done).resolves.toMatchObject({
      status: 'failed',
      error: expect.stringContaining('time limit'),
    })
    expect(worker.terminate).toHaveBeenCalledOnce()
  })

  it('aborts a call still waiting at the document barrier before it dispatches', async () => {
    const worker = new FakeWorker()
    const barrier = deferred<void>()
    const transport = vi.fn()
    const run = runJavaScript('', context, {
      call: async (_method, _params, signal) => {
        await barrier.promise
        signal.throwIfAborted()
        return transport()
      },
      workerFactory: () => worker,
    })
    worker.emit({ type: 'call', id: 1, method: 'session.list', params: {} })
    run.stop()
    barrier.resolve()
    await expect(run.done).resolves.toEqual({ status: 'stopped' })
    expect(transport).not.toHaveBeenCalled()
  })

  it.each([
    null,
    { type: 'call', id: 0, method: 'session.list', params: {} },
    { type: 'call', id: 1, method: 'automation.run_python', params: {} },
    { type: 'call', id: 1, method: 'doc.edit_begin', params: {} },
    { type: 'call', id: 1, method: 'doc.edit_apply', params: {} },
    { type: 'call', id: 1, method: 'doc.edit_end', params: {} },
    { type: 'call', id: 1, method: 'session.list', params: [] },
    { type: 'output', level: 'trace', text: 'bad' },
    { type: 'output', level: 'log', text: 42 },
    { type: 'failure', error: null },
    { type: 'unexpected' },
  ])('rejects malformed or recursive worker requests: %j', async (message) => {
    const worker = new FakeWorker()
    const run = runJavaScript('', context, {
      call,
      workerFactory: () => worker,
    })
    worker.emit(message)
    await expect(run.done).resolves.toMatchObject({ status: 'failed' })
    expect(call).not.toHaveBeenCalled()
  })

  it('rejects duplicate request identifiers', async () => {
    const worker = new FakeWorker()
    const run = runJavaScript('', context, {
      call,
      workerFactory: () => worker,
    })
    worker.emit({ type: 'call', id: 1, method: 'session.list', params: {} })
    worker.emit({ type: 'call', id: 1, method: 'session.list', params: {} })
    await expect(run.done).resolves.toMatchObject({ status: 'failed' })
    expect(call).toHaveBeenCalledTimes(1)
  })

  it('bounds UTF-8 source before creating a worker', async () => {
    const factory = vi.fn(() => new FakeWorker())
    const run = runJavaScript(
      '字'.repeat(MACRO_LIMITS.sourceBytes / 2),
      context,
      {
        call,
        workerFactory: factory,
      },
    )
    await expect(run.done).resolves.toMatchObject({ status: 'failed' })
    expect(factory).not.toHaveBeenCalled()
  })

  it('bounds output volume', async () => {
    const worker = new FakeWorker()
    const run = runJavaScript('', context, {
      call,
      workerFactory: () => worker,
    })
    for (let i = 0; i <= MACRO_LIMITS.outputLines; i++) {
      worker.emit({ type: 'output', level: 'log', text: 'x' })
    }
    await expect(run.done).resolves.toMatchObject({
      status: 'failed',
      error: expect.stringContaining('output limit'),
    })
  })

  it('rejects oversized RPC messages before dispatch', async () => {
    const worker = new FakeWorker()
    const run = runJavaScript('', context, {
      call,
      workerFactory: () => worker,
    })
    worker.emit({
      type: 'call',
      id: 1,
      method: 'session.list',
      params: { oversized: 'x'.repeat(MACRO_LIMITS.messageBytes) },
    })
    await expect(run.done).resolves.toMatchObject({ status: 'failed' })
    expect(call).not.toHaveBeenCalled()
  })

  it('bounds queued calls and still waits for the active request', async () => {
    const worker = new FakeWorker()
    const pending = deferred<unknown[]>()
    const bridge = vi.fn(() => pending.promise)
    const run = runJavaScript('', context, {
      call: bridge as MacroApiCall,
      workerFactory: () => worker,
    })
    for (let id = 1; id <= MACRO_LIMITS.calls + 1; id++) {
      worker.emit({ type: 'call', id, method: 'session.list', params: {} })
    }
    expect(worker.terminate).toHaveBeenCalledOnce()
    pending.resolve([])
    await expect(run.done).resolves.toMatchObject({
      status: 'failed',
      error: expect.stringContaining('call limit'),
    })
    expect(bridge).toHaveBeenCalledTimes(1)
  })

  it('does not accept premature completion with an active API request', async () => {
    const worker = new FakeWorker()
    const run = runJavaScript('', context, {
      call,
      workerFactory: () => worker,
    })
    worker.emit({ type: 'call', id: 1, method: 'session.list', params: {} })
    worker.emit({ type: 'done' })
    await expect(run.done).resolves.toMatchObject({
      status: 'failed',
      error: expect.stringContaining('unfinished'),
    })
  })

  it('reports worker startup and bridge failures without leaking rejected promises', async () => {
    const startup = runJavaScript('', context, {
      call,
      workerFactory: () => {
        throw new Error('blocked by CSP')
      },
    })
    await expect(startup.done).resolves.toEqual({
      status: 'failed',
      error: 'blocked by CSP',
    })
    const worker = new FakeWorker()
    const run = runJavaScript('', context, {
      call: vi.fn().mockRejectedValue(new Error('gone')) as MacroApiCall,
      workerFactory: () => worker,
    })
    worker.emit({ type: 'call', id: 1, method: 'session.list', params: {} })
    await vi.waitFor(() =>
      expect(worker.postMessage).toHaveBeenCalledWith({
        type: 'response',
        id: 1,
        error: 'gone',
      }),
    )
    worker.emit({ type: 'failure', error: 'gone' })
    await expect(run.done).resolves.toEqual({ status: 'failed', error: 'gone' })
  })
})

describe('macro worker execution', () => {
  function harness(script: MacroScript) {
    const sent: unknown[] = []
    const scope = {
      onmessage: null as ((event: MessageEvent<unknown>) => void) | null,
      postMessage: (message: unknown) => sent.push(message),
    }
    installMacroWorker(scope, async () => script)
    const receive = (message: unknown) =>
      scope.onmessage?.({ data: message } as MessageEvent<unknown>)
    receive({ type: 'start', source: '', context })
    return { sent, receive }
  }

  it('waits for fire-and-forget API work before reporting the return value and completion', async () => {
    const { sent, receive } = harness(async (fc, fontcarver, console) => {
      expect(fc).toBe(fontcarver)
      expect(fc.context).toEqual(context)
      expect(Object.isFrozen(fc.context)).toBe(true)
      void fc.call('session.list', {})
      console.log('hello', { answer: 42 })
      return 'finished'
    })
    await vi.waitFor(() =>
      expect(sent).toContainEqual({
        type: 'call',
        id: 1,
        method: 'session.list',
        params: {},
      }),
    )
    expect(sent).not.toContainEqual({ type: 'done' })
    receive({ type: 'response', id: 1, result: [] })
    await vi.waitFor(() => expect(sent).toContainEqual({ type: 'done' }))
    expect(sent).toContainEqual({
      type: 'output',
      level: 'log',
      text: 'hello {"answer":42}',
    })
    expect(sent).toContainEqual({
      type: 'output',
      level: 'result',
      text: 'finished',
    })
  })

  it('waits for calls created by a detached API continuation', async () => {
    const { sent, receive } = harness(async (fc) => {
      void fc.call('session.list', {}).then(() => fc.call('session.list', {}))
    })
    await vi.waitFor(() => expect(sent).toHaveLength(1))
    receive({ type: 'response', id: 1, result: [] })
    await vi.waitFor(() => expect(sent).toHaveLength(2))
    expect(sent.at(-1)).toMatchObject({ type: 'call', id: 2 })
    receive({ type: 'response', id: 2, result: [] })
    await vi.waitFor(() => expect(sent.at(-1)).toEqual({ type: 'done' }))
  })

  it('reports fire-and-forget API failures and thrown script errors', async () => {
    const api = harness(async (fc) => {
      void fc.call('session.list', {})
    })
    await vi.waitFor(() => expect(api.sent).toHaveLength(1))
    api.receive({ type: 'response', id: 1, error: 'Document closed' })
    await vi.waitFor(() =>
      expect(api.sent.at(-1)).toEqual({
        type: 'failure',
        error: 'Document closed',
      }),
    )
    const script = harness(async () => {
      throw new Error('bad macro')
    })
    await vi.waitFor(() =>
      expect(script.sent.at(-1)).toEqual({
        type: 'failure',
        error: 'bad macro',
      }),
    )
  })

  it('allows awaited and chained API errors to be handled by the macro', async () => {
    for (const chained of [false, true]) {
      const { sent, receive } = harness(async (fc) => {
        if (chained) {
          await fc
            .call('session.list', {})
            .then(() => 'unused')
            .catch(() => 'handled')
        } else {
          try {
            await fc.call('session.list', {})
          } catch {
            /* recovered */
          }
        }
        return 'recovered'
      })
      await vi.waitFor(() => expect(sent).toHaveLength(1))
      receive({ type: 'response', id: 1, error: 'Document closed' })
      await vi.waitFor(() => expect(sent.at(-1)).toEqual({ type: 'done' }))
      expect(sent).toContainEqual({
        type: 'output',
        level: 'result',
        text: 'recovered',
      })
    }
  })

  it('bounds API fan-out inside the worker before further requests can accumulate', async () => {
    const { sent } = harness(async (fc) => {
      for (let i = 0; i <= MACRO_LIMITS.calls; i++)
        void fc.call('session.list', {})
    })
    await vi.waitFor(() =>
      expect(sent.at(-1)).toMatchObject({
        type: 'failure',
        error: expect.stringContaining('call limit'),
      }),
    )
    expect(sent).toHaveLength(MACRO_LIMITS.calls + 1)
  })

  it('rejects unknown host responses', async () => {
    const { sent, receive } = harness(async (fc) => fc.call('session.list', {}))
    await vi.waitFor(() => expect(sent).toHaveLength(1))
    receive({ type: 'response', id: 9, result: [] })
    expect(sent.at(-1)).toEqual({
      type: 'failure',
      error: 'Unknown macro API response identifier',
    })
  })
})
