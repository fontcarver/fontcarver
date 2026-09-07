import { useEffect, useMemo, useState } from 'react'

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

  const coordinator = useMemo(
    () =>
      new TextShapeCoordinator(getTransport(), setState, {
        onError: reportError,
      }),
    [],
  )

  useEffect(() => () => coordinator.dispose(), [coordinator])

  useEffect(() => {
    if (!context || editing) return
    coordinator.update(context)
  }, [context, coordinator, editing])

  useEffect(() => {
    const failure = state?.newCompileFailure
    if (!failure) return
    pushToast({ kind: 'error', message: failure })
  }, [pushToast, state?.newCompileFailure])

  return state
}
