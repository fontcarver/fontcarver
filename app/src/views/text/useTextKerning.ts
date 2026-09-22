import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from 'react'
import { useTranslation } from 'react-i18next'
import { call, type DocId } from '../../api'
import { noteDocumentEdit } from '../../editor/editSession'
import { documentEditingSuspended } from '../../editor/documentEditParticipants'
import { reportError } from '../../shell/commands'
import { isEditableTarget } from '../../shell/shortcuts'
import { useUiStore } from '../../stores/uiStore'
import { ApiCallError } from '../../transport'
import {
  KerningEditRunner,
  kerningShortcut,
  type KerningEditTarget,
  type KerningShortcutEvent,
} from './kerningEdits'
import {
  kerningScopeRecord,
  kerningScopeSides,
  parseKerningValue,
  type KerningPairNames,
  type KerningScope,
  type TextKerningModel,
} from './kerningModel'
import { useKerningImpact } from './kerningImpact'
import { useStoredKerning } from './kerningQuery'

/** TextView supplies document identity and captures old controller work. */
export interface UseTextKerningOptions {
  doc: DocId
  vertical?: boolean
  master: string | null
  generation: number
  selectedPair?: KerningPairNames | null
  prepare?: () => Promise<void>
}

interface Draft {
  key: string
  target: KerningEditTarget
  pair: KerningPairNames
  scope: KerningScope
  value: string
}
interface Run {
  key: string
  runner: KerningEditRunner
  target: KerningEditTarget
  pair: KerningPairNames
  scope: KerningScope
}

/** Owns stored pair edits and their barriers; it never changes shaped geometry. */
export function useTextKerning(options: UseTextKerningOptions) {
  const { t } = useTranslation()
  const [pair, setPairState] = useState<KerningPairNames>(
    options.selectedPair ?? { left: '', right: '' },
  )
  const [scope, setScopeState] = useState<KerningScope>('glyph_glyph')
  const [draft, setDraftState] = useState<Draft | null>(null)
  const [invalid, setInvalid] = useState(false)
  const [editing, setEditing] = useState(false)
  const [revision, setRevision] = useState(0)
  const alive = useRef(true)
  const draftRef = useRef<Draft | null>(null)
  const runs = useRef(new Set<Run>())
  const currentRun = useRef<Run | null>(null)
  const changeId = useRef(0)
  const key = JSON.stringify([
    options.doc,
    options.master,
    options.vertical,
    pair.left,
    pair.right,
  ])
  const editKey = JSON.stringify([key, scope])
  const loaded = useStoredKerning(
    options.doc,
    options.master,
    options.generation,
    pair,
    revision,
    options.vertical,
  )
  const resolution = loaded?.resolution ?? null
  const status = loaded?.status ?? null
  const sides = resolution ? kerningScopeSides(resolution, scope) : null
  const impact = useKerningImpact(options.doc, resolution, scope)
  const editable = !!(
    status?.editable &&
    sides &&
    options.master &&
    !impact.loading &&
    !impact.failed
  )
  const exact = resolution ? kerningScopeRecord(resolution, scope) : null
  const value =
    draft?.key === editKey
      ? draft.value
      : String(exact?.value ?? resolution?.value ?? 0)
  const latest = useRef({
    options,
    pair,
    scope,
    key,
    editKey,
    resolution,
    editable,
    sides,
  })
  useLayoutEffect(() => {
    latest.current = {
      options,
      pair,
      scope,
      key,
      editKey,
      resolution,
      editable,
      sides,
    }
  })

  const refresh = useCallback(() => {
    if (alive.current) setRevision((value) => value + 1)
  }, [])

  const createRun = useCallback(
    (
      target: KerningEditTarget,
      capturedPair: KerningPairNames,
      capturedScope: KerningScope,
      capturedKey: string,
      removing = false,
    ): Run => {
      // prepare can synchronously invoke finish. The draft is cleared before this
      // call, and the new runner is registered afterwards, so it cannot await itself.
      const after = latest.current.options.prepare?.()
      const record: Run = {
        key: capturedKey,
        target,
        pair: capturedPair,
        scope: capturedScope,
        runner: new KerningEditRunner({
          ...target,
          after,
          label: t(removing ? 'kerningText.undoRemove' : 'kerningText.undo'),
          refreshStart: async () => {
            const fresh = await call(
              target.vertical ? 'kerning.resolve_vertical' : 'kerning.resolve',
              {
                doc: target.doc,
                master: target.master,
                ...capturedPair,
              },
            )
            if (!fresh.status.editable || fresh.value === null)
              throw new ApiCallError(
                'kerning_unavailable',
                'Kerning is unavailable',
              )
            const freshSides = kerningScopeSides(fresh, capturedScope)
            if (
              !freshSides ||
              JSON.stringify(freshSides.left) !== JSON.stringify(target.left) ||
              JSON.stringify(freshSides.right) !== JSON.stringify(target.right)
            )
              throw new ApiCallError(
                'stale_generation',
                'Kerning group membership changed',
              )
            return (
              kerningScopeRecord(fresh, capturedScope)?.value ?? fresh.value
            )
          },
          onEnd: () => {
            runs.current.delete(record)
            if (currentRun.current === record) currentRun.current = null
            if (alive.current) {
              setEditing(runs.current.size > 0)
              refresh()
            }
          },
          onError: reportError,
        }),
      }
      runs.current.add(record)
      currentRun.current = record
      if (alive.current) setEditing(true)
      return record
    },
    [refresh, t],
  )

  const closeRuns = useCallback(
    () =>
      Promise.allSettled([...runs.current].map((run) => run.runner.end())).then(
        () => undefined,
      ),
    [],
  )
  const commit = useCallback((): Promise<void> => {
    const pending = draftRef.current
    if (!pending) return closeRuns()
    const numeric = parseKerningValue(pending.value)
    draftRef.current = null
    if (alive.current) {
      setDraftState(null)
      setInvalid(numeric === null)
    }
    if (numeric === null) return closeRuns()
    // Publish every old closure before constructing the next session.
    void closeRuns()
    const run = createRun(
      pending.target,
      pending.pair,
      pending.scope,
      pending.key,
    )
    run.runner.set(numeric)
    return run.runner.end()
  }, [closeRuns, createRun])
  const finish = useCallback(() => {
    void commit()
    return closeRuns()
  }, [closeRuns, commit])

  useEffect(() => {
    alive.current = true
    return () => {
      alive.current = false
      void finish()
    }
  }, [finish])

  const setPair = useCallback(
    async (next: KerningPairNames) => {
      const request = ++changeId.current
      await finish()
      if (!alive.current || request !== changeId.current) return
      setPairState({ left: next.left.trim(), right: next.right.trim() })
      setScopeState('glyph_glyph')
      setInvalid(false)
    },
    [finish],
  )
  const setScope = useCallback(
    async (next: KerningScope) => {
      const request = ++changeId.current
      const captured = latest.current
      if (!captured.resolution || !kerningScopeSides(captured.resolution, next))
        return
      await finish()
      if (!alive.current || request !== changeId.current) return
      if (latest.current.key === captured.key) {
        setScopeState(next)
        setInvalid(false)
        refresh()
      }
    },
    [finish, refresh],
  )

  const selectedLeft = options.selectedPair?.left
  const selectedRight = options.selectedPair?.right
  const previousDoc = useRef(options.doc)
  useEffect(() => {
    const changedDoc = previousDoc.current !== options.doc
    previousDoc.current = options.doc
    if (selectedLeft && selectedRight)
      void setPair({ left: selectedLeft, right: selectedRight })
    else if (changedDoc) void setPair({ left: '', right: '' })
    else void finish()
  }, [
    options.doc,
    options.master,
    selectedLeft,
    selectedRight,
    setPair,
    finish,
  ])

  const setValue = useCallback((text: string) => {
    const current = latest.current
    if (documentEditingSuspended(current.options.doc)) return
    if (!current.editable || !current.sides || !current.options.master) return
    noteDocumentEdit(current.options.doc)
    const pending: Draft = {
      key: current.editKey,
      pair: current.pair,
      scope: current.scope,
      value: text,
      target: {
        doc: current.options.doc,
        master: current.options.master,
        vertical: current.options.vertical,
        ...current.sides,
      },
    }
    draftRef.current = pending
    setDraftState(pending)
    setInvalid(false)
  }, [])
  const cancel = useCallback(() => {
    draftRef.current = null
    setDraftState(null)
    setInvalid(false)
  }, [])
  const remove = useCallback(() => {
    const current = latest.current
    if (documentEditingSuspended(current.options.doc)) return Promise.resolve()
    if (!current.editable || !current.sides || !current.options.master)
      return Promise.resolve()
    // Removal replaces an unfinished numeric draft at this exact scope.
    cancel()
    void closeRuns()
    const run = createRun(
      {
        doc: current.options.doc,
        master: current.options.master,
        vertical: current.options.vertical,
        ...current.sides,
      },
      current.pair,
      current.scope,
      current.editKey,
      true,
    )
    run.runner.set(null)
    return run.runner.end()
  }, [cancel, closeRuns, createRun])

  const onKerningKey = useCallback(
    (event: KerningShortcutEvent): boolean => {
      const delta = kerningShortcut(
        event,
        /Mac|iPhone|iPad/u.test(navigator.platform),
      )
      const current = latest.current
      if (
        delta === null ||
        isEditableTarget(event.target ?? null) ||
        useUiStore.getState().modalCount > 0
      )
        return false
      if (!alive.current) return false
      if (documentEditingSuspended(current.options.doc)) return true
      const previous = currentRun.current
      if (previous?.key === current.editKey) {
        if (previous.runner.active) previous.runner.press(delta)
        else {
          void finish()
          createRun(
            previous.target,
            previous.pair,
            previous.scope,
            previous.key,
          ).runner.press(delta)
        }
        return true
      }
      if (!current.editable || !current.sides || !current.options.master)
        return false
      void finish()
      const run = createRun(
        {
          doc: current.options.doc,
          master: current.options.master,
          vertical: current.options.vertical,
          ...current.sides,
        },
        current.pair,
        current.scope,
        current.editKey,
      )
      run.runner.press(delta)
      return true
    },
    [createRun, finish],
  )

  const panel: TextKerningModel = {
    pair,
    scope,
    status,
    resolution,
    loading: loaded === null,
    loadFailed: loaded?.failed ?? false,
    editable,
    value,
    invalid,
    editing,
    impact: impact.groups,
    impactLoading: impact.loading,
    impactFailed: impact.failed,
    setPair,
    setScope,
    setValue,
    commit,
    cancel,
    remove,
  }
  return { finish, onKerningKey, panel }
}
