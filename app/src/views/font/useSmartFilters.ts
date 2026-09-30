import { useEffect, useRef, useState } from 'react'

import { call, onDocumentEvent, type ApiMethods, type DocId } from '../../api'
import { activeEditor } from '../../editor/activeEditor'
import {
  holdDocumentEdits,
  pendingDocumentEdits,
} from '../../editor/editSession'

/** A named expression stored with the font project. */
export type SavedSmartFilter =
  ApiMethods['glyph.smart_filters.get']['result']['filters'][number]

interface State {
  doc: DocId
  filters: SavedSmartFilter[]
  loaded: boolean
  busy: boolean
  error: 'loadError' | 'saveError' | null
}

interface Context {
  doc: DocId
  active: boolean
  revision: number
  busy: boolean
}

function initial(doc: DocId): State {
  return { doc, filters: [], loaded: false, busy: false, error: null }
}

/** Loads saved expressions and serializes their undoable project updates. */
export function useSmartFilters(doc: DocId) {
  const [state, setState] = useState<State>(() => initial(doc))
  const [reloadCount, setReloadCount] = useState(0)
  const context = useRef<Context | null>(null)

  useEffect(() => {
    const current: Context = { doc, active: true, revision: 0, busy: false }
    context.current = current
    const reload = () => {
      const revision = ++current.revision
      void call('glyph.smart_filters.get', { doc }).then(
        (answer) => {
          if (!current.active || current.revision !== revision) return
          setState({
            doc,
            filters: answer.filters,
            loaded: true,
            busy: current.busy,
            error: null,
          })
        },
        () => {
          if (!current.active || current.revision !== revision) return
          setState((previous) => ({
            ...(previous.doc === doc ? previous : initial(doc)),
            error: 'loadError',
          }))
        },
      )
    }
    reload()
    const unsubscribe = onDocumentEvent((event) => {
      if (event.doc === doc && event.change.font_info) reload()
    })
    return () => {
      current.active = false
      unsubscribe()
    }
  }, [doc, reloadCount])

  const save = async (filters: SavedSmartFilter[]): Promise<boolean> => {
    const current = context.current
    if (!current?.active || current.doc !== doc || current.busy) return false
    current.busy = true
    const revision = ++current.revision
    setState((previous) => ({ ...previous, busy: true, error: null }))
    // Capture the existing barrier before publishing this operation, so a save
    // cannot overtake it and the filter command cannot wait on itself.
    try {
      const settling = activeEditor()?.settle()
      const earlier = pendingDocumentEdits(doc)
      const work = Promise.all([settling, earlier]).then(() =>
        call('glyph.smart_filters.set', { doc, filters }),
      )
      holdDocumentEdits(doc, work)
      const answer = await work
      if (!current.active) return false
      if (current.revision === revision) {
        setState({
          doc,
          filters: answer.filters,
          loaded: true,
          busy: true,
          error: null,
        })
      }
      return true
    } catch (error) {
      if (current.active)
        setState((previous) => ({ ...previous, error: 'saveError' }))
      throw error
    } finally {
      current.busy = false
      if (current.active) setState((previous) => ({ ...previous, busy: false }))
    }
  }

  const retry = () => {
    setState((previous) => ({ ...previous, error: null }))
    setReloadCount((value) => value + 1)
  }

  return { ...(state.doc === doc ? state : initial(doc)), save, retry }
}
