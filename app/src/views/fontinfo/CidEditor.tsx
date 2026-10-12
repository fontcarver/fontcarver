import { useEffect, useState, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import { onDocumentEvent, type DocId } from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { reportError } from '../../stores/uiStore'
import { errorNotice } from '../../i18n/errors'
import { CidSession, identityMapping, unmappedGlyphs } from './cidSession'
import styles from './designSpace.module.css'

/** Staged CID identity and pasteable numeric mapping, applied as one undo step. */
export function CidEditor({
  doc,
  onClose,
}: {
  doc: DocId
  onClose: () => void
}) {
  const { t } = useTranslation()
  const [session] = useState(
    () => new CidSession(doc, t('cid.edit'), t('cid.invalid')),
  )
  const state = useSyncExternalStore(session.subscribe, session.snapshot)
  useEffect(() => {
    const dispose = session.connect()
    void session.load().catch(reportError)
    const off = onDocumentEvent((event) => {
      if (event.doc === doc) void session.load().catch(reportError)
    })
    return () => {
      off()
      dispose()
    }
  }, [session, doc])
  const disabled = state.busy || state.suspended
  const close = () => {
    void session.flush().then(onClose).catch(reportError)
  }
  const error = state.error ? errorNotice(state.error, t) : null
  const draft = state.draft
  // Export refuses a CID-keyed font with an incomplete mapping, so the gap is
  // reported here, where it can still be filled.
  const unmapped =
    state.source && draft
      ? unmappedGlyphs(state.source, draft, t('cid.invalid'))
      : []
  return (
    <Dialog
      title={t('cid.title')}
      size="lg"
      scrollBody
      onCancel={close}
      footer={
        <>
          <Button
            disabled={disabled}
            onClick={() => {
              void session.discard().then(onClose).catch(reportError)
            }}
          >
            {t('variation.discard')}
          </Button>
          <Button disabled={disabled} onClick={close}>
            {t('common.close')}
          </Button>
          <Button
            variant="primary"
            disabled={disabled || !state.dirty}
            onClick={() => {
              void session.flush().catch(reportError)
            }}
          >
            {t('variation.apply')}
          </Button>
        </>
      }
    >
      <div className={styles.editor}>
        {error && (
          <div role="alert">
            <p>{error.message}</p>
            {error.details && <p>{error.details}</p>}
            <Button
              disabled={disabled}
              onClick={() => {
                void session.discard().catch(reportError)
              }}
            >
              {t('variation.reload')}
            </Button>
          </div>
        )}
        {!draft ? (
          <p>{t('fontInfo.loading')}</p>
        ) : (
          <fieldset
            disabled={disabled || !state.source}
            className={styles.form}
          >
            <label>
              <input
                type="checkbox"
                checked={draft.enabled}
                onChange={(e) => session.edit({ enabled: e.target.checked })}
              />
              {t('cid.enabled')}
            </label>
            <fieldset disabled={!draft.enabled} className={styles.form}>
              <div className={styles.row}>
                {(['registry', 'ordering', 'supplement'] as const).map(
                  (field) => (
                    <label key={field}>
                      {t(`cid.${field}`)}
                      <input
                        value={draft[field]}
                        maxLength={field === 'supplement' ? 10 : 127}
                        onChange={(e) =>
                          session.edit({ [field]: e.target.value })
                        }
                      />
                    </label>
                  ),
                )}
              </div>
              <p>{t('cid.mappingHint')}</p>
              <label className={styles.mapping}>
                {t('cid.mapping')}
                <textarea
                  spellCheck={false}
                  rows={12}
                  maxLength={4_000_000}
                  value={draft.mapping}
                  onChange={(e) => session.edit({ mapping: e.target.value })}
                />
              </label>
              <div className={styles.row}>
                <Button
                  disabled={
                    draft.registry !== 'Adobe' || draft.ordering !== 'Identity'
                  }
                  onClick={() => {
                    if (!state.source) return
                    try {
                      session.edit({
                        mapping: identityMapping(
                          state.source,
                          draft,
                          t('cid.invalid'),
                        ),
                      })
                    } catch (error) {
                      reportError(error)
                    }
                  }}
                >
                  {t('cid.fillIdentity')}
                </Button>
                <label>
                  <input
                    type="checkbox"
                    checked={draft.reorder}
                    onChange={(e) =>
                      session.edit({ reorder: e.target.checked })
                    }
                  />
                  {t('cid.reorder')}
                </label>
              </div>
              {unmapped.length > 0 && (
                <p role="status">
                  {t('cid.unmapped', {
                    count: unmapped.length,
                    names: unmapped.slice(0, 6).join(', '),
                  })}
                </p>
              )}
              <p>{t('cid.identityHint')}</p>
            </fieldset>
          </fieldset>
        )}
      </div>
    </Dialog>
  )
}
