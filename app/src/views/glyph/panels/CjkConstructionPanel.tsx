import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { activeEditor } from '../../../editor/activeEditor'
import { commit } from '../../../editor/pointCommands'
import { reportError } from '../../../shell/commands'
import { useDocumentStore } from '../../../stores/documentStore'
import { IdsError, idsCommand } from './cjkConstruction'
import {
  CjkResolveError,
  resolveCjkIds,
  type CjkResolvedComponent,
} from './cjkResolve'
import { CjkRadicalLibrary } from './CjkRadicalLibrary'
import { CjkTransformGrid } from './CjkTransformGrid'
import type { PanelContext } from './panelContext'
import panelStyles from './CjkConstructionPanel.module.css'
import sharedStyles from './panels.module.css'

/** Build editable CJK references from document radicals and Unicode IDS input. */
export function CjkConstructionPanel({
  target,
  gid,
  scene,
  outline,
  master,
  reload,
}: PanelContext) {
  const { t } = useTranslation()
  const generation = useDocumentStore(
    (state) => state.byDoc[target.doc]?.generation ?? 0,
  )
  const [ids, setIds] = useState('')
  const [library, setLibrary] = useState(false)
  const [busy, setBusy] = useState(false)
  const [preview, setPreview] = useState<{
    key: string
    leaves: CjkResolvedComponent[]
  } | null>(null)
  const [error, setError] = useState<{ key: string; message: string } | null>(
    null,
  )
  const sequence = useRef(0)
  const { upem, descender } = scene.metrics
  const key = JSON.stringify([
    target.doc,
    target.glyph,
    target.layer,
    master,
    generation,
    outline.width,
    upem,
    descender,
    ids,
  ])
  const currentKey = useRef(key)
  // Event handlers keep their captured target; asynchronous previews must not
  // become actionable after navigation, edits, undo, or input changes.
  useEffect(() => {
    currentKey.current = key
    return () => {
      sequence.current += 1
    }
  }, [key])

  const leaves = preview?.key === key ? preview.leaves : null
  const makePreview = async () => {
    const request = ++sequence.current
    setBusy(true)
    setPreview(null)
    setError(null)
    try {
      const resolved = await resolveCjkIds(
        target.doc,
        master,
        target.glyph,
        ids,
        outline.width,
        upem,
        descender,
      )
      if (request === sequence.current && key === currentKey.current)
        setPreview({ key, leaves: resolved })
    } catch (failure) {
      if (request !== sequence.current || key !== currentKey.current) return
      const message =
        failure instanceof IdsError
          ? t(`cjkConstruction.errors.${failure.code}`)
          : failure instanceof CjkResolveError
            ? t(`cjkConstruction.errors.${failure.code}`, {
                token: failure.token,
              })
            : t('cjkConstruction.errors.resolve')
      setError({ key, message })
    } finally {
      setBusy(false)
    }
  }

  const apply = async () => {
    if (!leaves || busy) return
    setBusy(true)
    try {
      await activeEditor()?.settle()
      // Settling a live edit can change the document generation; require a
      // fresh preview instead of applying obsolete geometry.
      if (currentKey.current !== key) return
      await commit(
        target,
        idsCommand(
          target,
          leaves.map(({ component }) => component),
        ),
        t('cjkConstruction.undo'),
      )
      setPreview(null)
    } catch (failure) {
      reportError(failure)
      setError({ key, message: t('cjkConstruction.errors.apply') })
    } finally {
      setBusy(false)
      reload()
    }
  }

  return (
    <div className={panelStyles.panel}>
      <button
        className={sharedStyles.actionButton}
        type="button"
        aria-expanded={library}
        onClick={() => setLibrary(!library)}
      >
        {t('cjkConstruction.library')}
      </button>
      {library && (
        <CjkRadicalLibrary
          doc={target.doc}
          master={master}
          glyph={target.glyph}
          generation={generation}
          onPick={(token) => setIds((value) => `${value}${token}`)}
        />
      )}
      <label className={panelStyles.label}>
        {t('cjkConstruction.ids')}
        <input
          className={sharedStyles.textInput}
          value={ids}
          disabled={busy}
          placeholder={t('cjkConstruction.placeholder')}
          onChange={(event) => setIds(event.currentTarget.value)}
        />
      </label>
      <p className={panelStyles.hint}>{t('cjkConstruction.help')}</p>
      <div className={sharedStyles.actions}>
        <button
          className={sharedStyles.actionButton}
          type="button"
          disabled={!ids.trim() || busy}
          onClick={() => {
            void makePreview()
          }}
        >
          {t('cjkConstruction.preview')}
        </button>
        <button
          className={sharedStyles.actionButton}
          type="button"
          disabled={!leaves || busy}
          onClick={() => {
            void apply()
          }}
        >
          {t('cjkConstruction.append')}
        </button>
      </div>
      {busy && <p role="status">{t('cjkConstruction.loading')}</p>}
      {error?.key === key && <p role="alert">{error.message}</p>}
      {leaves && (
        <>
          <svg
            className={panelStyles.preview}
            viewBox={`0 ${-(descender + upem)} ${outline.width} ${upem}`}
            role="img"
            aria-label={t('cjkConstruction.previewLabel')}
          >
            <g transform="scale(1 -1)">
              {leaves.map(({ component, cell }, index) => (
                <path
                  key={index}
                  d={cell.path ?? undefined}
                  transform={`matrix(${component.transform.join(' ')})`}
                />
              ))}
            </g>
          </svg>
          <p className={panelStyles.hint}>
            {t('cjkConstruction.previewHint', { count: leaves.length })}
          </p>
          <ol className={panelStyles.names}>
            {leaves.map(({ component }, index) => (
              <li key={index}>{component.base}</li>
            ))}
          </ol>
        </>
      )}
      <CjkTransformGrid
        target={target}
        gid={gid}
        scene={scene}
        outline={outline}
        reload={reload}
      />
    </div>
  )
}
