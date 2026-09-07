import { useEffect, useRef, useState } from 'react'

import { getTransport } from '../../api'
import { reportError } from '../../shell/commands'
import { useEditorStore } from '../../stores/editorStore'
import { useUiStore } from '../../stores/uiStore'

import {
  TextShapeCoordinator,
  type TextRequestContext,
  type TextShapeState,
} from './shapeCoordinator'

/** Connects the controller-independent coordinator to React and edit deferral. */
export function useTextShape(
  context: TextRequestContext | null,
): TextShapeState | null {
  const [state, setState] = useState<TextShapeState | null>(null)
  const editing = useEditorStore((store) => store.editing)
  const pushToast = useUiStore((store) => store.pushToast)

  const coordinatorRef = useRef<TextShapeCoordinator | null>(null)
  // Create each effect lifetime's own coordinator: StrictMode intentionally
  // disposes and restarts effects once when the development app mounts.
  useEffect(() => {
    const coordinator = new TextShapeCoordinator(getTransport(), setState, {
      onError: reportError,
    })
    coordinatorRef.current = coordinator
    return () => {
      coordinator.dispose()
      coordinatorRef.current = null
    }
  }, [])

  useEffect(() => {
    const coordinator = coordinatorRef.current
    if (!coordinator) return
    if (!context || editing) {
      coordinator.suspend()
      return
    }
    coordinator.update(context)
  }, [context, editing])

  useEffect(() => {
    const failure = state?.newCompileFailure
    if (!failure) return
    pushToast({ kind: 'error', message: failure })
  }, [pushToast, state?.newCompileFailure])

  // Never label geometry from a different text/master/direction as current
  // while the replacement request is still in its typing debounce.
  if (!context || !state) return null
  const previous = state.context
  if (
    previous.doc !== context.doc ||
    previous.master !== context.master ||
    previous.text !== context.text ||
    previous.direction !== context.direction ||
    JSON.stringify(previous.features) !== JSON.stringify(context.features)
  )
    return null
  return state
}
