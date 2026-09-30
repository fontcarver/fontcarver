import type { ApiMethods } from '../api/generated'
import {
  MACRO_LIMITS,
  byteLength,
  errorText,
  isContext,
  isWorkerRequest,
  withinMessageLimit,
  type MacroContext,
  type MacroMethod,
  type MacroOutput,
  type WorkerRequest,
  type WorkerResponse,
} from './protocol'

export type {
  MacroApiCall,
  MacroContext,
  MacroFacade,
  MacroOutput,
} from './protocol'

/** Minimal worker contract permits deterministic lifecycle tests. */
export interface MacroWorker {
  onmessage: ((event: MessageEvent<unknown>) => void) | null
  onerror: ((event: ErrorEvent) => void) | null
  onmessageerror: ((event: MessageEvent<unknown>) => void) | null
  postMessage(message: WorkerResponse): void
  terminate(): void
}

export interface MacroResult {
  status: 'completed' | 'stopped' | 'failed'
  error?: string
}

export interface MacroRun {
  /** Settles only after the last dispatched API call has settled. */
  done: Promise<MacroResult>
  stop(): void
}

export interface MacroOptions {
  call: MacroHostCall
  onOutput?: (output: MacroOutput) => void
  workerFactory?: () => MacroWorker
  /** Defaults to ten seconds and cannot exceed thirty seconds. */
  timeoutMs?: number
}

/** The signal lets document barriers discard calls not yet sent to the kernel. */
export type MacroHostCall = <M extends MacroMethod>(
  method: M,
  params: ApiMethods[M]['params'],
  signal: AbortSignal,
) => Promise<ApiMethods[M]['result']>

/** Runs trusted local JavaScript in a terminable worker, not a security sandbox. */
export function runJavaScript(
  source: string,
  context: MacroContext,
  options: MacroOptions,
): MacroRun {
  let resolve!: (result: MacroResult) => void
  const done = new Promise<MacroResult>((complete) => (resolve = complete))
  const controller = new AbortController()
  let worker: MacroWorker | undefined
  let terminal: MacroResult | undefined
  let timer: ReturnType<typeof setTimeout> | undefined
  let inFlight = false
  let nextId = 1
  let outputLines = 0
  let outputBytes = 0
  const queue: Extract<WorkerRequest, { type: 'call' }>[] = []

  const finish = (result: MacroResult) => {
    if (terminal) return
    terminal = result
    controller.abort()
    clearTimeout(timer)
    queue.length = 0
    if (worker) {
      worker.onmessage = null
      worker.onerror = null
      worker.onmessageerror = null
      worker.terminate()
    }
    if (!inFlight) resolve(result)
  }
  const fail = (error: unknown) =>
    finish({ status: 'failed', error: errorText(error) })

  const send = (response: WorkerResponse) => {
    if (terminal) return
    if (!withinMessageLimit(response)) {
      fail('Macro API response exceeded the message limit')
      return
    }
    try {
      worker?.postMessage(response)
    } catch (error) {
      fail(error)
    }
  }

  const dispatch = async () => {
    if (inFlight || terminal) return
    const request = queue.shift()
    if (!request) return
    inFlight = true
    try {
      const result = await options.call(
        request.method,
        request.params as ApiMethods[typeof request.method]['params'],
        controller.signal,
      )
      send({ type: 'response', id: request.id, result })
    } catch (error) {
      send({ type: 'response', id: request.id, error: errorText(error) })
    } finally {
      inFlight = false
      if (terminal) resolve(terminal)
      else void dispatch()
    }
  }

  const receive = (event: MessageEvent<unknown>) => {
    if (terminal) return
    if (!isWorkerRequest(event.data)) {
      fail('Invalid macro worker message or recursive automation call')
      return
    }
    const message = event.data
    switch (message.type) {
      case 'call':
        if (message.id !== nextId++ || message.id > MACRO_LIMITS.calls) {
          fail('Macro API call limit exceeded or invalid request identifier')
          return
        }
        queue.push(message)
        void dispatch()
        break
      case 'output':
        outputLines++
        outputBytes += byteLength(message.text)
        if (
          outputLines > MACRO_LIMITS.outputLines ||
          outputBytes > MACRO_LIMITS.outputBytes
        ) {
          fail('Macro output limit exceeded')
          return
        }
        try {
          options.onOutput?.({ level: message.level, text: message.text })
        } catch (error) {
          fail(error)
        }
        break
      case 'failure':
        fail(message.error)
        break
      case 'done':
        if (inFlight || queue.length)
          fail('Macro completed with unfinished API calls')
        else finish({ status: 'completed' })
        break
    }
  }

  const run: MacroRun = { done, stop: () => finish({ status: 'stopped' }) }
  if (
    typeof source !== 'string' ||
    byteLength(source) > MACRO_LIMITS.sourceBytes
  ) {
    fail('Macro source exceeds the 64 KiB limit')
    return run
  }
  if (!isContext(context)) {
    fail('Invalid macro selection context')
    return run
  }
  try {
    worker =
      options.workerFactory?.() ??
      new Worker(new URL('./macro.worker.ts', import.meta.url), {
        type: 'module',
        name: 'fontcarver-macro',
      })
    worker.onmessage = receive
    worker.onerror = (event) => {
      event.preventDefault?.()
      fail(event.message || 'Macro worker failed')
    }
    worker.onmessageerror = () =>
      fail('Macro worker message could not be decoded')
    const requestedTimeout = options.timeoutMs ?? MACRO_LIMITS.timeoutMs
    const timeout = Number.isFinite(requestedTimeout)
      ? Math.max(1, Math.min(requestedTimeout, MACRO_LIMITS.maxTimeoutMs))
      : MACRO_LIMITS.timeoutMs
    timer = setTimeout(() => fail('Macro exceeded its time limit'), timeout)
    send({ type: 'start', source, context: { ...context } })
  } catch (error) {
    fail(error)
  }
  return run
}
