import { useEffect, useId, useState, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import { onDocumentEvent, type DocId } from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { settleDocumentEdits } from '../../editor/documentPersistence'
import { errorNotice } from '../../i18n/errors'
import { reportError } from '../../stores/uiStore'
import {
  FontMetadataSession,
  type FontMetadataField,
} from './fontMetadataSession'
import styles from './fontMetadata.module.css'

const IDENTITY_FIELDS = [
  ['family_name', 'familyName'],
  ['style_name', 'styleName'],
  ['open_type_name_preferred_family_name', 'preferredFamily'],
  ['open_type_name_preferred_subfamily_name', 'preferredSubfamily'],
  ['open_type_name_wws_family_name', 'wwsFamily'],
  ['open_type_name_wws_subfamily_name', 'wwsSubfamily'],
  ['open_type_name_compatible_full_name', 'compatibleFull'],
  ['postscript_font_name', 'postscriptName'],
  ['postscript_full_name', 'postscriptFullName'],
  ['open_type_name_unique_id', 'uniqueId'],
  ['open_type_name_version', 'versionString'],
] as const satisfies readonly (readonly [FontMetadataField, string])[]
const AUTHOR_FIELDS = [
  ['open_type_name_designer', 'designer'],
  ['open_type_name_designer_url', 'designerUrl'],
  ['open_type_name_manufacturer', 'manufacturer'],
  ['open_type_name_manufacturer_url', 'manufacturerUrl'],
  ['open_type_name_license_url', 'licenseUrl'],
] as const satisfies readonly (readonly [FontMetadataField, string])[]
const TEXT_FIELDS = [
  ['copyright', 'copyright'],
  ['trademark', 'trademark'],
  ['open_type_name_license', 'license'],
  ['open_type_name_description', 'description'],
  ['open_type_name_sample_text', 'sampleText'],
  ['note', 'note'],
] as const satisfies readonly (readonly [FontMetadataField, string])[]

/** Stages existing font metadata; applying UPM changes never scale outlines. */
export function FontMetadataEditor({
  doc,
  onClose,
}: {
  doc: DocId
  onClose: () => void
}) {
  const { t } = useTranslation()
  const hintId = useId()
  const [session] = useState(
    () => new FontMetadataSession(doc, t('fontMetadata.edit')),
  )
  const state = useSyncExternalStore(session.subscribe, session.snapshot)
  useEffect(() => {
    const dispose = session.connect()
    void session.load().catch(reportError)
    const off = onDocumentEvent((event) => {
      if (event.doc === doc && event.change.font_info)
        void session.load().catch(reportError)
    })
    return () => {
      off()
      dispose()
    }
  }, [doc, session])
  const disabled = state.busy || state.suspended
  const apply = () => {
    void settleDocumentEdits(doc).catch(reportError)
  }
  const close = () => {
    if (disabled) return
    void settleDocumentEdits(doc).then(onClose).catch(reportError)
  }
  const error = state.error ? errorNotice(state.error, t) : null
  const textField = (field: FontMetadataField, label: string) => (
    <label key={field}>
      {t(`fontInfo.field.${label}`)}
      <input
        value={state.draft?.[field] ?? ''}
        onChange={(event) => session.edit({ [field]: event.target.value })}
      />
    </label>
  )
  return (
    <Dialog
      title={t('fontMetadata.title')}
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
              onClick={() => {
                void session.discard().catch(reportError)
              }}
            >
              {t('variation.reload')}
            </Button>
          </div>
        )}
        {!state.draft ? (
          <p>{t('fontInfo.loading')}</p>
        ) : (
          <fieldset disabled={disabled} className={styles.form}>
            <legend>{t('fontMetadata.identity')}</legend>
            <div className={styles.grid}>
              {IDENTITY_FIELDS.map(([field, label]) => textField(field, label))}
              <label>
                {t('fontInfo.field.unitsPerEm')}
                <input
                  inputMode="numeric"
                  aria-describedby={hintId}
                  value={state.draft.units_per_em}
                  onChange={(event) =>
                    session.edit({ units_per_em: event.target.value })
                  }
                />
              </label>
              {(['version_major', 'version_minor'] as const).map((field) => (
                <label key={field}>
                  {t(`fontMetadata.${field}`)}
                  <input
                    inputMode="numeric"
                    value={state.draft?.[field] ?? ''}
                    onChange={(event) =>
                      session.edit({ [field]: event.target.value })
                    }
                  />
                </label>
              ))}
            </div>
            <p id={hintId}>{t('fontMetadata.upmHint')}</p>
            <p>{t('fontMetadata.namesHint')}</p>
            <h3>{t('fontMetadata.authorAndLegal')}</h3>
            <div className={styles.grid}>
              {AUTHOR_FIELDS.map(([field, label]) => textField(field, label))}
            </div>
            <div className={styles.textFields}>
              {TEXT_FIELDS.map(([field, label]) => (
                <label key={field}>
                  {t(`fontInfo.field.${label}`)}
                  <textarea
                    rows={3}
                    value={state.draft?.[field] ?? ''}
                    onChange={(event) =>
                      session.edit({ [field]: event.target.value })
                    }
                  />
                </label>
              ))}
            </div>
          </fieldset>
        )}
      </div>
    </Dialog>
  )
}
