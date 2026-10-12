import { call, type ApiMethods } from '../api'
import { withSettledDocument } from '../editor/documentPersistence'
import { useDocumentStore } from '../stores/documentStore'
import { useEditorStore } from '../stores/editorStore'
import { activeDocId } from '../stores/uiStore'
import type { MacroContext } from './runtime'
import { isMacroMethod } from './protocol'

/** Snapshots the active editor context once, before an explicit run. */
export function currentMacroContext(): MacroContext {
  const doc = activeDocId()
  const view = doc === null ? undefined : useDocumentStore.getState().byDoc[doc]
  const active = useEditorStore.getState().activeGlyph
  return {
    doc,
    master: active?.doc === doc ? active.master : (view?.master ?? null),
    glyph: active?.doc === doc ? active.gid : (view?.focus ?? null),
  }
}

/** Settles pending editor drafts before each macro RPC involving a document. */
export function macroCall<M extends keyof ApiMethods>(
  method: M,
  params: ApiMethods[M]['params'],
  signal?: AbortSignal,
): Promise<ApiMethods[M]['result']> {
  if (!isMacroMethod(method))
    return Promise.reject(
      new Error(
        'Recursive automation and raw gesture transactions are unavailable',
      ),
    )
  const record = typeof params === 'object' && params !== null ? params : {}
  const docs = [
    ...new Set(
      [
        'doc' in record ? record.doc : null,
        'source_doc' in record ? record.source_doc : null,
      ].filter((value): value is number => typeof value === 'number'),
    ),
  ].sort((a, b) => a - b)
  const invoke = () => {
    if (signal?.aborted)
      return Promise.reject(new Error('Macro execution has stopped'))
    return call(method, params)
  }
  const settle = (index: number): Promise<ApiMethods[M]['result']> =>
    index === docs.length
      ? invoke()
      : withSettledDocument(docs[index], () => settle(index + 1))
  return settle(0)
}
