import { useEffect, useState, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import { onDocumentEvent, type DocId } from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { errorNotice } from '../../i18n/errors'
import { reportError } from '../../stores/uiStore'
import { SvgSession } from './svgSession'
import { newDocument } from './svgDraft'
import { SvgPreview } from './SvgPreview'
import styles from './svg.module.css'

/** Source and named glyph bindings for static OpenType SVG documents. */
export function SvgEditor({
  doc,
  onClose,
}: {
  doc: DocId
  onClose: () => void
}) {
  const { t } = useTranslation()
  const [session] = useState(
    () => new SvgSession(doc, { edit: t('svg.edit'), decode: t('svg.decode') }),
  )
  const state = useSyncExternalStore(session.subscribe, session.snapshot)
  const [selected, setSelected] = useState(0)
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
  }, [doc, session])
  const disabled = state.busy || state.suspended
  const close = () => {
    if (disabled) return
    if (!state.dirty) onClose()
    else void session.flush().then(onClose).catch(reportError)
  }
  const { draft, source } = state
  const index = Math.min(
    selected,
    Math.max(0, (draft?.documents.length ?? 1) - 1),
  )
  const current = draft?.documents[index]
  const update = (patch: Partial<NonNullable<typeof current>>) => {
    if (draft)
      session.edit({
        documents: draft.documents.map((d, i) =>
          i === index ? { ...d, ...patch } : d,
        ),
      })
  }
  const error =
    state.error instanceof Error &&
    state.error.message === 'svg.invalidBindings'
      ? { message: t('svg.invalidBindings'), details: null }
      : state.error
        ? errorNotice(state.error, t)
        : null
  return (
    <Dialog
      title={t('svg.title')}
      size="lg"
      scrollBody
      onCancel={close}
      footer={
        <>
          <Button
            disabled={disabled}
            onClick={() =>
              void session.discard().then(onClose).catch(reportError)
            }
          >
            {t('variation.discard')}
          </Button>
          <Button disabled={disabled} onClick={close}>
            {t('common.close')}
          </Button>
          <Button
            variant="primary"
            disabled={disabled || !state.dirty}
            onClick={() => void session.flush().catch(reportError)}
          >
            {t('variation.apply')}
          </Button>
        </>
      }
    >
      <div className={styles.editor}>
        <p>{t('svg.description')}</p>
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
        ) : (
          <fieldset disabled={disabled}>
            {draft === null ? (
              <>
                <p>{t(`svg.status.${source.status}`)}</p>
                {source.import_error && (
                  <p role="note">{source.import_error}</p>
                )}
                <div className={styles.row}>
                  {source.status === 'Imported' && (
                    <Button
                      disabled={state.dirty || !!source.import_error}
                      onClick={() => void session.decode().catch(reportError)}
                    >
                      {t('svg.decode')}
                    </Button>
                  )}
                  <Button
                    onClick={() => session.edit({ documents: [newDocument()] })}
                  >
                    {t(
                      source.status === 'Imported'
                        ? 'svg.replaceImported'
                        : 'svg.create',
                    )}
                  </Button>
                  {source.status === 'Imported' && (
                    <Button onClick={() => session.edit(null)}>
                      {t('svg.delete')}
                    </Button>
                  )}
                </div>
                {state.dirty && <p>{t('svg.pendingDeletion')}</p>}
              </>
            ) : (
              <>
                <div className={styles.row}>
                  <Button
                    onClick={() => {
                      session.edit({
                        documents: [...draft.documents, newDocument()],
                      })
                      setSelected(draft.documents.length)
                    }}
                  >
                    {t('svg.add')}
                  </Button>
                  <Button
                    disabled={!current}
                    onClick={() =>
                      session.edit({
                        documents: draft.documents.filter(
                          (_, i) => i !== index,
                        ),
                      })
                    }
                  >
                    {t('svg.remove')}
                  </Button>
                  <Button onClick={() => session.edit(null)}>
                    {t('svg.delete')}
                  </Button>
                </div>
                {current && (
                  <>
                    <div className={styles.row}>
                      <label>
                        {t('svg.document')}
                        <input
                          aria-label={t('svg.document')}
                          type="number"
                          min={1}
                          max={draft.documents.length}
                          value={index + 1}
                          onChange={(e) => {
                            const next = e.currentTarget.valueAsNumber
                            if (
                              Number.isInteger(next) &&
                              next >= 1 &&
                              next <= draft.documents.length
                            )
                              setSelected(next - 1)
                          }}
                        />
                      </label>
                      <span>
                        {t('svg.count', { count: draft.documents.length })}
                      </span>
                    </div>
                    <label>
                      {t('svg.bindings')}
                      <textarea
                        rows={3}
                        spellCheck={false}
                        value={current.bindings}
                        onChange={(e) => update({ bindings: e.target.value })}
                      />
                    </label>
                    <p>{t('svg.bindingHelp')}</p>
                    <label>
                      {t('svg.source')}
                      <textarea
                        rows={12}
                        spellCheck={false}
                        value={current.source}
                        onChange={(e) => update({ source: e.target.value })}
                      />
                    </label>
                    <SvgPreview
                      source={current.source}
                      bindings={current.bindings}
                      unitsPerEm={source.units_per_em}
                    />
                  </>
                )}
              </>
            )}
          </fieldset>
        )}
      </div>
    </Dialog>
  )
}
