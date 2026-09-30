import {
  MACRO_LIMITS,
  byteLength,
  errorText,
  isContext,
  isMacroMethod,
  isRecord,
  withinMessageLimit,
  type MacroApiCall,
  type MacroContext,
  type MacroFacade,
  type MacroLevel,
  type WorkerRequest,
} from './protocol'

export type MacroConsole = Record<
  Exclude<MacroLevel, 'result'>,
  (...values: unknown[]) => void
>
export type MacroScript = (
  fc: MacroFacade,
  fontcarver: MacroFacade,
  console: MacroConsole,
) => Promise<unknown>

export interface MacroWorkerScope {
  onmessage: ((event: MessageEvent<unknown>) => void) | null
  postMessage(message: WorkerRequest): void
}

async function loadScript(source: string): Promise<MacroScript> {
  // A module URL avoids unsafe-eval. The desktop CSP explicitly permits blob:
  // scripts; macros remain trusted local code with normal worker capabilities.
  const url = URL.createObjectURL(
    new Blob(
      [
        `export default async function(fc, fontcarver, console) {\n${source}\n}\n`,
      ],
      { type: 'text/javascript' },
    ),
  )
  try {
    const module = (await import(/* @vite-ignore */ url)) as {
      default: MacroScript
    }
    return module.default
  } finally {
    URL.revokeObjectURL(url)
  }
}

function format(value: unknown): string {
  if (typeof value === 'string') return value
  if (value instanceof Error) return value.message
  try {
    return JSON.stringify(value) ?? String(value)
  } catch {
    return String(value)
  }
}

/** Installs one-run worker handling; the injectable loader is only used in tests. */
export function installMacroWorker(
  scope: MacroWorkerScope,
  loader: (source: string) => Promise<MacroScript> = loadScript,
): void {
  let started = false
  let ended = false
  let nextId = 1
  let outputLines = 0
  let outputBytes = 0
  const pending = new Map<
    number,
    {
      resolve(value: unknown): void
      reject(error: Error): void
    }
  >()
  const allCalls: { promise: Promise<unknown>; observed: boolean }[] = []

  // A Promise-compatible thenable lets await, catch, and chained handlers mark
  // the original rejection as observed. Native backing promises let us drain
  // every detached branch without turning a caught API error into run failure.
  const track = <T>(promise: Promise<T>): Promise<T> => {
    const work = { promise, observed: false }
    allCalls.push(work)
    void promise.catch(() => undefined)
    return {
      [Symbol.toStringTag]: 'Promise',
      then<TResult1 = T, TResult2 = never>(
        onfulfilled?: ((value: T) => TResult1 | PromiseLike<TResult1>) | null,
        onrejected?:
          ((reason: unknown) => TResult2 | PromiseLike<TResult2>) | null,
      ): Promise<TResult1 | TResult2> {
        work.observed = true
        return track(promise.then(onfulfilled, onrejected))
      },
      catch<TResult = never>(
        onrejected?:
          ((reason: unknown) => TResult | PromiseLike<TResult>) | null,
      ): Promise<T | TResult> {
        work.observed = true
        return track(promise.catch(onrejected))
      },
      finally(onfinally?: (() => void) | null): Promise<T> {
        work.observed = true
        return track(promise.finally(onfinally))
      },
    }
  }

  const fail = (error: unknown) => {
    if (ended) return
    ended = true
    scope.postMessage({ type: 'failure', error: errorText(error) })
  }
  const send = (message: WorkerRequest) => {
    if (ended) throw new Error('Macro execution has ended')
    if (!withinMessageLimit(message))
      throw new Error('Macro message limit exceeded')
    scope.postMessage(message)
  }
  const output = (level: MacroLevel, values: unknown[]) => {
    const text = values.map(format).join(' ')
    outputLines++
    outputBytes += byteLength(text)
    if (
      outputLines > MACRO_LIMITS.outputLines ||
      outputBytes > MACRO_LIMITS.outputBytes
    ) {
      const error = new Error('Macro output limit exceeded')
      fail(error)
      throw error
    }
    send({ type: 'output', level, text })
  }
  const apiCall = (method: unknown, params: unknown): Promise<unknown> => {
    if (ended) throw new Error('Macro execution has ended')
    if (!isMacroMethod(method) || !isRecord(params)) {
      const error = new Error('Invalid API call or recursive automation method')
      fail(error)
      throw error
    }
    if (nextId > MACRO_LIMITS.calls) {
      const error = new Error('Macro API call limit exceeded')
      fail(error)
      throw error
    }
    const id = nextId++
    const promise = new Promise<unknown>((resolve, reject) => {
      pending.set(id, { resolve, reject })
      try {
        send({ type: 'call', id, method, params })
      } catch (error) {
        pending.delete(id)
        reject(error)
      }
    })
    return track(promise)
  }

  const execute = async (source: string, context: MacroContext) => {
    let result: unknown
    let failure: unknown
    let failed = false
    try {
      const script = await loader(source)
      const facade = Object.freeze({
        context: Object.freeze({ ...context }),
        call: apiCall as MacroApiCall,
      })
      const console: MacroConsole = Object.freeze({
        log: (...values: unknown[]) => output('log', values),
        info: (...values: unknown[]) => output('info', values),
        warn: (...values: unknown[]) => output('warn', values),
        error: (...values: unknown[]) => output('error', values),
      })
      result = await script(facade, facade, console)
    } catch (error) {
      failed = true
      failure = error
    }
    // Recheck after each batch: promise continuations may enqueue more API work.
    let drained = 0
    const settlements: PromiseSettledResult<unknown>[] = []
    while (drained < allCalls.length && !ended) {
      const batch = allCalls.slice(drained)
      drained += batch.length
      settlements.push(
        ...(await Promise.allSettled(batch.map((work) => work.promise))),
      )
    }
    for (const [index, settlement] of settlements.entries()) {
      if (
        settlement.status === 'rejected' &&
        !allCalls[index].observed &&
        !failed
      ) {
        failed = true
        failure = settlement.reason
      }
    }
    if (ended) return
    if (failed) {
      fail(failure)
      return
    }
    if (result !== undefined) output('result', [result])
    send({ type: 'done' })
    ended = true
  }

  scope.onmessage = (event) => {
    if (ended) return
    const message = event.data
    if (!isRecord(message) || !withinMessageLimit(message)) {
      fail('Invalid macro host message')
      return
    }
    if (message.type === 'start') {
      if (
        started ||
        typeof message.source !== 'string' ||
        byteLength(message.source) > MACRO_LIMITS.sourceBytes ||
        !isContext(message.context)
      ) {
        fail('Invalid macro start message')
        return
      }
      started = true
      void execute(message.source, message.context).catch(fail)
      return
    }
    if (
      message.type !== 'response' ||
      !Number.isSafeInteger(message.id) ||
      (message.error !== undefined && typeof message.error !== 'string')
    ) {
      fail('Invalid macro API response')
      return
    }
    const request = pending.get(message.id as number)
    if (!request) {
      fail('Unknown macro API response identifier')
      return
    }
    pending.delete(message.id as number)
    if (typeof message.error === 'string')
      request.reject(new Error(message.error))
    else request.resolve(message.result)
  }
}

if (
  typeof document === 'undefined' &&
  typeof globalThis.postMessage === 'function'
) {
  installMacroWorker(globalThis)
}
