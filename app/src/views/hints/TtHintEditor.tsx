import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type DocId,
  type TtHintDraft,
  type TtHintPreview,
  type TtHintState,
} from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { activeEditor } from '../../editor/activeEditor'
import { errorNotice } from '../../i18n/errors'
import { TtGlobalFields } from './TtGlobalFields'
import { TtPointPreview } from './TtPointPreview'
import { TtPreviewResults } from './TtPreviewResults'
import { ttDraft, ttNeedsReview } from './ttDraft'
import styles from './hints.module.css'

export function TtHintEditor({
  doc,
  master,
  glyph,
  layer,
  onClose,
  onApplied,
}: {
  doc: DocId
  master: string
  glyph: string
  layer: string
  onClose: () => void
  onApplied: () => void
}) {
  const { t } = useTranslation()
  const [state, setState] = useState<TtHintState | null>(null)
  const [draft, setDraft] = useState<TtHintDraft | null>(null)
  const [cvt, setCvt] = useState('')
  const [reviewed, setReviewed] = useState(false)
  const [ppems, setPpems] = useState('12, 16, 24')
  const [mono, setMono] = useState(true)
  const [preview, setPreview] = useState<TtHintPreview | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const form = useRef<HTMLFormElement>(null)
  const mounted = useRef(true)
  const translate = useRef(t)
  useEffect(() => {
    translate.current = t
  }, [t])
  useEffect(() => {
    let current = true
    mounted.current = true
    void Promise.resolve(activeEditor()?.settle())
      .then(() => call('hints.tt_state', { doc, master, glyph }))
      .then((result) => {
        if (!current) return
        if (result.layer !== layer) {
          setError(translate.current('tt.masterOnly'))
          return
        }
        setState(result)
        setDraft(ttDraft(result))
        setCvt(result.globals.cvt.join(', '))
      })
      .catch((reason: unknown) => {
        if (current) {
          const notice = errorNotice(reason, translate.current)
          setError(notice.details ?? notice.message)
        }
      })
    return () => {
      current = false
      mounted.current = false
    }
  }, [doc, master, glyph, layer])
  const report = (reason: unknown) => {
    if (!mounted.current) return
    const notice = errorNotice(reason, t)
    setError(notice.details ?? notice.message)
  }
  const change = (value: TtHintDraft) => {
    setDraft(value)
    setPreview(null)
  }
  const needsReview = state ? ttNeedsReview(state) : false
  const ready = !!state && !!draft && (!needsReview || reviewed) && !busy
  const run = (save: boolean) => {
    if (!ready || !state || !draft || !form.current?.reportValidity()) return
    const sizes = ppems
      .trim()
      .split(/[,\s]+/)
      .map(Number)
    if (
      !save &&
      (sizes.length > 8 ||
        sizes.some((size) => !Number.isInteger(size) || size < 1 || size > 128))
    ) {
      setError(t('tt.invalidSizes'))
      return
    }
    const params = {
      target: { doc, master, glyph },
      expected_generation: state.generation,
      draft,
    }
    setBusy(true)
    setError('')
    setPreview(null)
    const request = save
      ? call('hints.tt_apply', { ...params, label: t('tt.editLabel') }).then(
          () => {
            if (mounted.current) {
              onApplied()
              onClose()
            }
          },
        )
      : call('hints.tt_preview', { ...params, ppems: sizes, mono }).then(
          (result) => {
            if (mounted.current) setPreview(result)
          },
        )
    void request.catch(report).finally(() => {
      if (mounted.current) setBusy(false)
    })
  }
  return (
    <Dialog
      title={t('tt.title', { glyph })}
      size="lg"
      scrollBody
      onCancel={() => {
        if (!busy) onClose()
      }}
      footer={
        <>
          <Button disabled={busy} onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button variant="primary" disabled={!ready} onClick={() => run(true)}>
            {t('variation.apply')}
          </Button>
        </>
      }
    >
      {error && <p role="alert">{error}</p>}
      {!state || !draft ? (
        !error && <p>{t('glyphView.loading')}</p>
      ) : (
        <form
          ref={form}
          className={styles.editor}
          onSubmit={(event) => {
            event.preventDefault()
            run(true)
          }}
        >
          <p>{t('tt.scope')}</p>
          <TtPointPreview state={state} />
          <fieldset disabled={busy}>
            <legend>{t('tt.source')}</legend>
            <p>{t('tt.assemblyHelp')}</p>
            {needsReview && (
              <>
                <p role="status">{t('tt.reviewHelp')}</p>
                <label className={styles.row}>
                  <input
                    type="checkbox"
                    checked={reviewed}
                    onChange={(event) => setReviewed(event.target.checked)}
                  />
                  {t('tt.reviewed')}
                </label>
              </>
            )}
            {!state.binding_matches &&
              !state.authored &&
              state.program.bytes.length > 0 && (
                <p>{t('tt.importedReference')}</p>
              )}
            {(['program', 'fpgm', 'prep'] as const).map((key) => (
              <div key={key}>
                {(state[key].bytes.length > 0 || state[key].error) && (
                  <details>
                    <summary>
                      {t('tt.original', { program: t(`tt.${key}`) })}
                    </summary>
                    {state[key].error && <p>{state[key].error}</p>}
                    <textarea
                      className={styles.assembly}
                      aria-label={t('tt.original', { program: t(`tt.${key}`) })}
                      rows={4}
                      readOnly
                      value={
                        state[key].assembly ??
                        state[key].bytes
                          .map((byte) => byte.toString(16).padStart(2, '0'))
                          .join(' ')
                      }
                    />
                  </details>
                )}
                <label className={styles.sourceField}>
                  {t(`tt.${key}`)}
                  <textarea
                    className={styles.assembly}
                    rows={key === 'program' ? 8 : 4}
                    spellCheck={false}
                    maxLength={2 * 1024 * 1024}
                    value={draft[key]}
                    onChange={(event) =>
                      change({ ...draft, [key]: event.target.value })
                    }
                  />
                </label>
              </div>
            ))}
            <TtGlobalFields
              value={draft.globals}
              cvt={cvt}
              onCvt={(value) => {
                setCvt(value)
                setPreview(null)
              }}
              onChange={(globals) => change({ ...draft, globals })}
            />
            <div className={styles.row}>
              <label>
                {t('tt.sizes')}
                <input
                  value={ppems}
                  onChange={(event) => {
                    setPpems(event.target.value)
                    setPreview(null)
                  }}
                />
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={mono}
                  onChange={(event) => {
                    setMono(event.target.checked)
                    setPreview(null)
                  }}
                />
                {t('tt.mono')}
              </label>
              <Button disabled={!ready} onClick={() => run(false)}>
                {t('tt.preview')}
              </Button>
            </div>
            <p>{t('tt.previewHelp')}</p>
          </fieldset>
          {busy && <p role="status">{t('tt.working')}</p>}
          {preview && <TtPreviewResults preview={preview} />}
        </form>
      )}
    </Dialog>
  )
}
