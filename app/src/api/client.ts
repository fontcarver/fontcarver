import { createTransport, type Transport } from '../transport'

import type { ApiMethods, DocumentEvent, ProgressEvent } from './generated'

let transport: Transport | undefined

/**
 * Replaces the process-wide transport. Tests inject a mock through this hook;
 * passing `undefined` restores lazy creation of the real transport.
 */
export function setTransport(next: Transport | undefined): void {
  transport = next
}

/** Returns the active transport, creating the environment default on first use. */
export function getTransport(): Transport {
  transport ??= createTransport()
  return transport
}

/** Calls one typed kernel method through the active transport. */
export function call<M extends keyof ApiMethods>(
  method: M,
  params: ApiMethods[M]['params'],
): Promise<ApiMethods[M]['result']> {
  return getTransport().call(method, params)
}

declare global {
  /**
   * Dev-only bridge to the kernel, used by the end-to-end suite and by
   * interactive verification to set up a document — creating a font, adding a
   * glyph, reading an outline back — where the interface has no surface for it
   * yet. It compiles out of a production build.
   */
  var __fwCall: typeof call | undefined
}

if (import.meta.env.DEV) globalThis.__fwCall = call

/** Subscribes to kernel document events; returns an unsubscribe function. */
export function onDocumentEvent(
  cb: (event: DocumentEvent) => void,
): () => void {
  return getTransport().onEvent((event) => {
    if (event.kind === 'Document') cb(event)
  })
}

/** Subscribes to long-running operation progress; returns an unsubscribe function. */
export function onProgressEvent(
  cb: (event: ProgressEvent) => void,
): () => void {
  return getTransport().onEvent((event) => {
    if (event.kind === 'Progress') cb(event)
  })
}
