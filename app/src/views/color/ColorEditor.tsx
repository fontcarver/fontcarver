import { useEffect, useRef, useState, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import { onDocumentEvent, type DocId } from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { errorNotice } from '../../i18n/errors'
import { reportError } from '../../stores/uiStore'
import { ColorPalettes } from './ColorPalettes'
import { ColorGlyphsPanel } from './ColorGlyphsPanel'
import { emptyTable } from './colorModel'
import { ColorSession } from './colorSession'
import styles from './color.module.css'

/** Shared color palettes and layered glyph compositions, applied as one undo step. */
export function ColorEditor({
  doc,
  onClose,
}: {
  doc: DocId
  onClose: () => void
}) {
  const { t } = useTranslation()
  const [session] = useState(
    () =>
      new ColorSession(doc, {
        edit: t('color.edit'),
        decode: t('color.decode'),
      }),
  )
  const state = useSyncExternalStore(session.subscribe, session.snapshot)
  const [palette, setPalette] = useState(0)
  const form = useRef<HTMLFormElement>(null)
  useEffect(() => {
    const dispose = session.connect(() => {
      // Close handovers suspend the form. Disabled controls are excluded from
      // reportValidity(), but their unparsed text still needs validation.
      const invalid = [...(form.current?.querySelectorAll('input') ?? [])].find(
        (input) =>
          !input.validity.valid ||
          (input.required && input.value.trim() === ''),
      )
      if (invalid) {
        invalid.reportValidity()
        throw new Error(
          invalid.dataset.validationMessage || invalid.validationMessage,
        )
      }
    })
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
    if (disabled || !valid()) return
    if (!state.dirty) return onClose()
    void session.flush().then(onClose).catch(reportError)
  }
  const error = state.error ? errorNotice(state.error, t) : null
  const source = state.source
  const draft = state.draft
  return (
    <Dialog
      title={t('color.title')}
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
            <p>{t(`color.status.${source.status}`)}</p>
            {source.status === 'Imported' && source.import_error && (
              <p role="note">
                {t('color.importError', { error: source.import_error })}
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
                  {t('color.decode')}
                </Button>
              )}
              <Button
                disabled={disabled}
                onClick={() => session.edit(emptyTable())}
              >
                {t(
                  source.status === 'Imported'
                    ? 'color.replaceImported'
                    : 'color.create',
                )}
              </Button>
              {source.status === 'Imported' && (
                <Button
                  disabled={disabled || state.dirty}
                  onClick={() => {
                    if (valid()) session.edit(null)
                  }}
                >
                  {t('color.deleteImported')}
                </Button>
              )}
            </div>
            {state.dirty && <p>{t('color.pendingDeletion')}</p>}
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
                <span className={styles.spacer} />
                <Button
                  onClick={() => {
                    if (valid()) session.edit(null)
                  }}
                >
                  {t('color.delete')}
                </Button>
              </div>
              <ColorPalettes
                table={draft}
                selected={Math.min(palette, draft.palettes.length - 1)}
                onSelect={setPalette}
                validate={valid}
                onChange={(next) => session.edit(next)}
              />
              <ColorGlyphsPanel
                doc={doc}
                table={draft}
                palette={Math.min(palette, draft.palettes.length - 1)}
                generation={source.generation}
                validate={valid}
                onChange={(next) => session.edit(next)}
              />
            </fieldset>
          </form>
        )}
      </div>
    </Dialog>
  )
}
