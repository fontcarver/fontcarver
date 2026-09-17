import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  call,
  type LayoutConversionPreview,
  type LayoutInspection,
  type MasterId,
} from '../../api'
import { withSettledDocument } from '../../editor/documentPersistence'
import { suspendDocumentEditParticipants } from '../../editor/documentEditParticipants'

import styles from './LookupInspector.module.css'

/** Revision-bound controls for the explicit original-to-source transition. */
export interface LayoutConversionProps {
  inspection: LayoutInspection
  master: MasterId | null
  onChanged: () => void
  beforeChange?: () => Promise<void>
}

/** Checks the exact conversion candidate before offering a single-use apply. */
export function LayoutConversion({
  inspection,
  master,
  onChanged,
  beforeChange,
}: LayoutConversionProps) {
  const { t } = useTranslation()
  const identity = JSON.stringify([
    inspection.doc,
    inspection.generation,
    inspection.compile_revision,
    inspection.draft_revision,
    inspection.fingerprint,
    inspection.authority,
    master,
  ])
  const latest = useRef({ identity, operation: 0, mounted: true })
  const [context, setContext] = useState(identity)
  const [preview, setPreview] = useState<LayoutConversionPreview | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [confirmRevert, setConfirmRevert] = useState(false)
  const [expired, setExpired] = useState(false)
  const [preferModelLift, setPreferModelLift] = useState(true)
  if (context !== identity) {
    setContext(identity)
    setPreview(null)
    setBusy(false)
    setError(null)
    setConfirmRevert(false)
    setExpired(false)
  }
  useEffect(() => {
    const live = latest.current
    live.identity = identity
    live.operation += 1
    live.mounted = true
    return () => {
      live.mounted = false
    }
  }, [identity])
  useEffect(() => {
    if (!preview?.token) return
    const timer = setTimeout(
      () => setExpired(true),
      preview.expires_in_seconds * 1000,
    )
    return () => clearTimeout(timer)
  }, [preview])

  const expected = {
    doc: inspection.doc,
    expected_generation: inspection.generation,
    expected_compile_revision: inspection.compile_revision,
    expected_draft_revision: inspection.draft_revision,
  }
  const run = (action: (current: () => boolean) => Promise<void>) => {
    if (busy) return
    const operation = ++latest.current.operation
    const current = () =>
      latest.current.mounted &&
      latest.current.identity === identity &&
      latest.current.operation === operation
    setBusy(true)
    setError(null)
    void withSettledDocument(inspection.doc, async () => {
      await beforeChange?.()
      if (!current()) return
      // Flush composition before making remounted source editors read-only.
      const resume = suspendDocumentEditParticipants(inspection.doc)
      try {
        await action(current)
      } finally {
        resume()
      }
    })
      .catch((reason: unknown) => {
        if (current()) {
          setError(
            reason instanceof Error
              ? reason.message
              : t('layoutInspector.conversion.failed'),
          )
          setPreview(null)
        }
      })
      .finally(() => {
        if (current()) setBusy(false)
      })
  }
  const prepare = () =>
    run(async (current) => {
      setPreview(null)
      setExpired(false)
      const result = await call('layout.prepare_conversion', {
        ...expected,
        master,
        prefer_model_lift: preferModelLift,
      })
      if (!current()) return
      if (
        result.doc !== inspection.doc ||
        result.generation !== inspection.generation ||
        result.compile_revision !== inspection.compile_revision ||
        result.draft_revision !== inspection.draft_revision ||
        result.fingerprint !== inspection.fingerprint ||
        (master !== null && result.master !== master)
      ) {
        throw new Error(t('layoutInspector.conversion.stale'))
      }
      setPreview(result)
    })
  const apply = () => {
    const token = preview?.token
    if (!token || expired) return
    run(async (current) => {
      await call('layout.apply_conversion', { doc: inspection.doc, token })
      if (current()) {
        setPreview(null)
        onChanged()
      }
    })
  }
  const revert = () =>
    run(async (current) => {
      await call('layout.revert_original', expected)
      if (current()) {
        setConfirmRevert(false)
        onChanged()
      }
    })
  const imported = inspection.authority === 'ImportedTables'
  if (!imported && !inspection.original_layout_available) return null

  return (
    <div className={styles.conversion} aria-busy={busy}>
      {imported && (
        <>
          <p className={styles.hint}>
            {t('layoutInspector.conversion.explanation')}
          </p>
          <label className={styles.controls}>
            <input
              type="checkbox"
              checked={preferModelLift}
              disabled={busy}
              onChange={(event) => {
                setPreferModelLift(event.currentTarget.checked)
                setPreview(null)
                setExpired(false)
              }}
            />
            {t('layoutInspector.conversion.preferModelLift')}
          </label>
          <button
            type="button"
            className={styles.button}
            disabled={busy || !inspection.can_convert}
            onClick={prepare}
          >
            {t(
              busy
                ? 'layoutInspector.conversion.working'
                : 'layoutInspector.conversion.prepare',
            )}
          </button>
          {!inspection.can_convert && (
            <p className={styles.hint}>
              {t('layoutInspector.conversion.blocked')}
            </p>
          )}
          {preview && (
            <>
              {preview.blockers.length > 0 && (
                <ul className={styles.list} role="alert">
                  {preview.blockers.map((blocker, index) => (
                    <li key={index}>{blocker}</li>
                  ))}
                </ul>
              )}
              {preview.messages.length > 0 && (
                <details>
                  <summary>{t('features.details')}</summary>
                  <ul className={styles.list}>
                    {preview.messages.map((message, index) => (
                      <li key={index} className={styles.hint}>
                        {message}
                      </li>
                    ))}
                  </ul>
                </details>
              )}
              {preview.token && (
                <>
                  <p className={styles.hint}>
                    {t('layoutInspector.conversion.proven')}
                  </p>
                  <p className={styles.hint}>
                    {t(
                      preview.kerning_lifted
                        ? 'layoutInspector.conversion.kerningLifted'
                        : 'layoutInspector.conversion.kerningSource',
                    )}
                  </p>
                  <p className={styles.hint}>
                    {t(
                      preview.anchors_lifted
                        ? 'layoutInspector.conversion.anchorsLifted'
                        : 'layoutInspector.conversion.anchorsSource',
                    )}
                  </p>
                  {expired && (
                    <p className={styles.alert} role="status">
                      {t('layoutInspector.conversion.expired')}
                    </p>
                  )}
                  <button
                    type="button"
                    className={styles.button}
                    disabled={busy || expired}
                    onClick={apply}
                  >
                    {t('layoutInspector.conversion.apply')}
                  </button>
                </>
              )}
            </>
          )}
        </>
      )}
      {!imported && inspection.original_layout_available && (
        <>
          <p className={styles.hint}>
            {t('layoutInspector.conversion.revertExplanation')}
          </p>
          {!confirmRevert && (
            <button
              type="button"
              className={styles.button}
              disabled={busy}
              onClick={() => setConfirmRevert(true)}
            >
              {t('layoutInspector.conversion.revert')}
            </button>
          )}
          {confirmRevert && (
            <>
              <p className={styles.hint}>
                {t('layoutInspector.conversion.revertConfirm')}
              </p>
              <div className={styles.controls}>
                <button
                  type="button"
                  className={styles.button}
                  disabled={busy}
                  onClick={revert}
                >
                  {t('layoutInspector.conversion.confirmRevert')}
                </button>
                <button
                  type="button"
                  className={styles.button}
                  disabled={busy}
                  onClick={() => setConfirmRevert(false)}
                >
                  {t('layoutInspector.conversion.cancel')}
                </button>
              </div>
            </>
          )}
        </>
      )}
      {error !== null && (
        <p className={styles.alert} role="alert">
          {t('layoutInspector.conversion.failed')} {error}
        </p>
      )}
    </div>
  )
}
