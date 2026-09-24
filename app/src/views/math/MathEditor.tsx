import { useEffect, useRef, useState, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import { onDocumentEvent, type DocId } from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { errorNotice } from '../../i18n/errors'
import { reportError } from '../../stores/uiStore'
import { MathConstantsForm } from './MathConstantsForm'
import { MathGlyphsPanel } from './MathGlyphsPanel'
import { emptyTable } from './mathModel'
import { MathSession } from './mathSession'
import styles from './math.module.css'

type Tab = 'constants' | 'glyphs'

/** Font-wide MATH constants, glyph records and constructions, applied as one undo step. */
export function MathEditor({
  doc,
  onClose,
}: {
  doc: DocId
  onClose: () => void
}) {
  const { t } = useTranslation()
  const [session] = useState(
    () =>
      new MathSession(doc, { edit: t('math.edit'), decode: t('math.decode') }),
  )
  const state = useSyncExternalStore(session.subscribe, session.snapshot)
  const [tab, setTab] = useState<Tab>('constants')
  const form = useRef<HTMLFormElement>(null)
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
  const valid = () => form.current?.reportValidity() ?? true
  const apply = () => {
    if (valid()) void session.flush().catch(reportError)
  }
  const close = () => {
    if (!state.dirty) return onClose()
    if (valid()) void session.flush().then(onClose).catch(reportError)
  }
  const error = state.error ? errorNotice(state.error, t) : null
  const source = state.source
  const draft = state.draft
  return (
    <Dialog
      title={t('math.title')}
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
            onClick={apply}
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
              onClick={() => void session.discard().catch(reportError)}
            >
              {t('variation.reload')}
            </Button>
          </div>
        )}
        {!source ? (
          <p>{t('fontInfo.loading')}</p>
        ) : draft === null ? (
          <div className={styles.section}>
            <p>{t(`math.status.${source.status}`)}</p>
            {source.status === 'Imported' && source.import_error && (
              <p role="note">
                {t('math.importError', { error: source.import_error })}
              </p>
            )}
            <div className={styles.row}>
              {source.status === 'Imported' && (
                <Button
                  disabled={
                    disabled || state.dirty || source.import_error !== null
                  }
                  onClick={() => void session.decode().catch(reportError)}
                >
                  {t('math.decode')}
                </Button>
              )}
              <Button
                disabled={disabled}
                onClick={() => session.edit(emptyTable())}
              >
                {t(
                  source.status === 'Imported'
                    ? 'math.replaceImported'
                    : 'math.create',
                )}
              </Button>
              {source.status === 'Imported' && (
                <Button
                  disabled={disabled || state.dirty}
                  onClick={() => session.edit(null)}
                >
                  {t('math.deleteImported')}
                </Button>
              )}
            </div>
            {state.dirty && <p>{t('math.pendingDeletion')}</p>}
          </div>
        ) : (
          <form
            ref={form}
            onSubmit={(event) => {
              event.preventDefault()
              apply()
            }}
          >
            <fieldset disabled={disabled} className={styles.form}>
              <div className={styles.row}>
                <span role="tablist" aria-label={t('math.sections')}>
                  {(['constants', 'glyphs'] as const).map((id) => (
                    <button
                      key={id}
                      type="button"
                      role="tab"
                      aria-selected={tab === id}
                      className={styles.tab}
                      onClick={() => setTab(id)}
                    >
                      {t(`math.tab.${id}`)}
                    </button>
                  ))}
                </span>
                <span className={styles.spacer} />
                <Button onClick={() => session.edit(null)}>
                  {t('math.delete')}
                </Button>
              </div>
              {tab === 'constants' ? (
                <MathConstantsForm
                  table={draft}
                  onChange={(next) => session.edit(next)}
                />
              ) : (
                <MathGlyphsPanel
                  doc={doc}
                  table={draft}
                  upem={source.units_per_em}
                  onChange={(next) => session.edit(next)}
                />
              )}
            </fieldset>
          </form>
        )}
      </div>
    </Dialog>
  )
}
