import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call, onDocumentEvent, type Command } from '../../../api'
import type { LayerTarget } from '../../../editor/pointCommands'
import { selectDocView, useDocumentStore } from '../../../stores/documentStore'
import { reportError } from '../../../shell/commands'

import { commitKerningGroup } from './kerningGroupEdits'
import { KerningGroupSideEditor } from './KerningGroupSideEditor'
import styles from './panels.module.css'
import { useKerningGroupQuery } from './useKerningGroupQuery'

export interface KerningGroupsPanelProps {
  target: LayerTarget
}

/** Shared group memberships for the glyph selected in the outline inspector. */
export function KerningGroupsPanel({ target }: KerningGroupsPanelProps) {
  const { t } = useTranslation()
  const generation = useDocumentStore(selectDocView(target.doc)).generation
  const [revision, setRevision] = useState(0)
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState(false)
  const [context, setContext] = useState(target)
  const mounted = useRef(true)
  // Reset local editing state before painting a new document or glyph.
  if (context.doc !== target.doc || context.glyph !== target.glyph) {
    setContext(target)
    setBusy(false)
    setFailed(false)
  }
  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
    }
  }, [])
  useEffect(
    () =>
      onDocumentEvent((event) => {
        if (event.doc === target.doc) setRevision((value) => value + 1)
      }),
    [target.doc],
  )
  const identity = `${target.doc}:${target.glyph}:${generation}:${revision}`
  const answer = useKerningGroupQuery(identity, () =>
    call('kerning.glyph', { doc: target.doc, glyph: target.glyph }),
  )
  const current = answer?.value
  const refresh = () => setRevision((value) => value + 1)
  // Async completions may refresh only the context that started the mutation.
  const latest = useRef({ target, operation: 0 })
  useEffect(() => {
    if (
      latest.current.target.doc !== target.doc ||
      latest.current.target.glyph !== target.glyph
    )
      latest.current.operation += 1
    latest.current.target = target
  }, [target])
  const run = (command: Command, label: string) => {
    const operation = ++latest.current.operation
    setBusy(true)
    setFailed(false)
    const stillCurrent = () =>
      mounted.current &&
      latest.current.operation === operation &&
      latest.current.target.doc === target.doc &&
      latest.current.target.glyph === target.glyph
    void commitKerningGroup(target.doc, command, label)
      .catch((error: unknown) => {
        reportError(error)
        if (stillCurrent()) setFailed(true)
      })
      .finally(() => {
        if (stillCurrent()) {
          setBusy(false)
          refresh()
        }
      })
  }

  if (!current)
    return (
      <>
        <p className={styles.hint} role={answer?.failed ? 'alert' : 'status'}>
          {t(
            answer?.failed
              ? 'kerningGroups.loadFailed'
              : 'kerningGroups.loading',
          )}
        </p>
        {answer?.failed && (
          <button
            className={styles.actionButton}
            type="button"
            onClick={refresh}
          >
            {t('kerningGroups.refresh')}
          </button>
        )}
      </>
    )

  return (
    <>
      <p className={styles.hint}>{t('kerningGroups.shared')}</p>
      {!current.status.editable && current.status.reason && (
        <p className={styles.hint}>
          {t(`kerningGroups.unavailable.${current.status.reason}`)}
        </p>
      )}
      {current.diagnostics.length > 0 && (
        <p
          className={styles.hint}
          role="alert"
          title={current.diagnostics.join('\n')}
        >
          {t('kerningGroups.invalidMembership')}
        </p>
      )}
      {failed && (
        <p className={styles.hint} role="alert">
          {t('kerningGroups.changeFailed')}
        </p>
      )}
      {(['side1', 'side2'] as const).map((side) => (
        <KerningGroupSideEditor
          key={`${target.doc}:${target.glyph}:${side}:${current.status.generation}`}
          doc={target.doc}
          glyph={target.glyph}
          side={side}
          membership={current[side]}
          status={current.status}
          busy={busy}
          run={run}
          refresh={refresh}
        />
      ))}
    </>
  )
}
