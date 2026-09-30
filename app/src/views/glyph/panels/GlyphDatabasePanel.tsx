import { useEffect, useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type GlyphDataState } from '../../../api'
import { Button } from '../../../components/Button'
import { errorNotice } from '../../../i18n/errors'
import { useDocumentStore } from '../../../stores/documentStore'
import styles from './GlyphDatabasePanel.module.css'
import type { PanelContext } from './panelContext'

interface Snapshot {
  doc: number
  gid: number
  value: GlyphDataState
}

interface Draft {
  doc: number
  generation: number
  text: string
  saved: string
}

interface Failure {
  doc: number
  gid: number
  cause: unknown
}

/** Effective glyph database defaults and the document's custom JSON overlay. */
export function GlyphDatabasePanel({ target, gid, reload }: PanelContext) {
  const { t } = useTranslation()
  const doc = target.doc
  const id = useId()
  const generation = useDocumentStore(
    (state) => state.byDoc[doc]?.generation ?? 0,
  )
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null)
  const [draft, setDraft] = useState<Draft | null>(null)
  const [refresh, setRefresh] = useState(0)
  const [pending, setPending] = useState<{ doc: number; gid: number } | null>(
    null,
  )
  const [error, setError] = useState<Failure | null>(null)
  const [loadError, setLoadError] = useState<Failure | null>(null)
  const operation = useRef(0)
  useEffect(() => {
    return () => {
      operation.current += 1
      setPending(null)
    }
  }, [doc, gid])

  useEffect(() => {
    let disposed = false
    void call('glyphdata.state', { doc, gid })
      .then((value) => {
        if (disposed) return
        setSnapshot({ doc, gid, value })
        setDraft((current) =>
          current?.doc === doc
            ? current
            : {
                doc,
                generation: value.generation,
                text: value.overlay_json,
                saved: value.overlay_json,
              },
        )
        setLoadError(null)
      })
      .catch((cause: unknown) => {
        if (!disposed) setLoadError({ doc, gid, cause })
      })
    return () => {
      disposed = true
    }
  }, [doc, gid, generation, refresh])

  const current =
    snapshot?.doc === doc && snapshot.gid === gid ? snapshot.value : null
  const editor = draft?.doc === doc ? draft : null
  const busy = pending?.doc === doc && pending.gid === gid
  const dirty = editor !== null && editor.text !== editor.saved
  const stale =
    editor !== null &&
    Math.max(generation, current?.generation ?? 0) > editor.generation
  const failure = [error, loadError].find(
    (value) => value?.doc === doc && value.gid === gid,
  )
  const notice = failure ? errorNotice(failure.cause, t) : null

  const reloadSaved = () => {
    setDraft(null)
    setError(null)
    setRefresh((value) => value + 1)
  }

  const mutate = async (save: boolean) => {
    if (!current || !editor || busy) return
    if (save) {
      try {
        JSON.parse(editor.text)
      } catch {
        setError({ doc, gid, cause: new Error(t('glyphDatabase.invalidJson')) })
        return
      }
    }
    const request = ++operation.current
    setPending({ doc, gid })
    setError(null)
    try {
      if (save) {
        await call('glyphdata.set_overlay', {
          doc,
          expected_generation: editor.generation,
          overlay_json: editor.text,
          label: t('glyphDatabase.editLabel'),
        })
      } else {
        await call('glyphdata.update', {
          doc,
          glyphs: [gid],
          expected_generation: current.generation,
          label: t('glyphDatabase.update'),
        })
      }
      if (request !== operation.current) return
      if (save) setDraft(null)
      setRefresh((value) => value + 1)
      reload()
    } catch (cause) {
      if (request === operation.current) setError({ doc, gid, cause })
    } finally {
      if (request === operation.current) setPending(null)
    }
  }

  const importFile = async (file: File | undefined) => {
    if (!file || !editor || busy) return
    if (file.size > 4 * 1024 * 1024) {
      setError({ doc, gid, cause: new Error(t('glyphDatabase.fileTooLarge')) })
      return
    }
    const request = ++operation.current
    setPending({ doc, gid })
    setError(null)
    try {
      const text = await file.text()
      if (request !== operation.current) return
      setDraft({ ...editor, text })
    } catch (cause) {
      if (request === operation.current) setError({ doc, gid, cause })
    } finally {
      if (request === operation.current) setPending(null)
    }
  }

  return (
    <details className={styles.section} open>
      <summary className={styles.title}>{t('glyphDatabase.title')}</summary>
      {notice && (
        <div role="alert" className={styles.error}>
          <p>{notice.message}</p>
          {notice.details && <p>{notice.details}</p>}
        </div>
      )}
      {!current ? (
        <p>{t('glyphDatabase.loading')}</p>
      ) : (
        <>
          <span className={styles.badge}>
            {t(
              current.custom ? 'glyphDatabase.custom' : 'glyphDatabase.builtIn',
            )}
          </span>
          <dl className={styles.metadata}>
            <dt>{t('glyphDatabase.category')}</dt>
            <dd>{current.info.category ?? t('glyphDatabase.none')}</dd>
            <dt>{t('glyphDatabase.subCategory')}</dt>
            <dd>{current.info.sub_category ?? t('glyphDatabase.none')}</dd>
            <dt>{t('glyphDatabase.script')}</dt>
            <dd>{current.info.script ?? t('glyphDatabase.none')}</dd>
            <dt>{t('glyphDatabase.productionName')}</dt>
            <dd>{current.info.production_name ?? t('glyphDatabase.none')}</dd>
            <dt>{t('glyphDatabase.anchors')}</dt>
            <dd>
              {current.info.anchors.join(', ') || t('glyphDatabase.none')}
            </dd>
            <dt>{t('glyphDatabase.composition')}</dt>
            <dd>
              {current.info.composition.join(' + ') || t('glyphDatabase.none')}
            </dd>
          </dl>
          <p className={styles.description}>
            {t('glyphDatabase.updateDescription')}
          </p>
          <Button disabled={busy || !editor} onClick={() => void mutate(false)}>
            {t('glyphDatabase.update')}
          </Button>
        </>
      )}
      <details className={styles.overlay} open>
        <summary>{t('glyphDatabase.overlayTitle')}</summary>
        <p id={`${id}-description`} className={styles.description}>
          {t('glyphDatabase.overlayDescription')}
        </p>
        <details>
          <summary>{t('glyphDatabase.schemaTitle')}</summary>
          <p className={styles.description}>
            {t('glyphDatabase.schemaDescription')}
          </p>
          <pre className={styles.example}>
            {JSON.stringify(
              {
                version: 1,
                glyphs: {
                  aacute: {
                    production_name: 'uni00E1',
                    anchors: ['top', 'bottom'],
                    composition: ['a', 'acutecomb'],
                  },
                },
              },
              null,
              2,
            )}
          </pre>
        </details>
        <label className={styles.label} htmlFor={`${id}-json`}>
          {t('glyphDatabase.json')}
        </label>
        <textarea
          id={`${id}-json`}
          className={styles.editor}
          aria-describedby={`${id}-description`}
          spellCheck={false}
          rows={6}
          value={editor?.text ?? ''}
          disabled={busy || !editor}
          onChange={(event) => {
            if (editor) setDraft({ ...editor, text: event.currentTarget.value })
          }}
        />
        <label className={styles.label} htmlFor={`${id}-file`}>
          {t('glyphDatabase.import')}
        </label>
        <input
          id={`${id}-file`}
          className={styles.file}
          type="file"
          accept=".json,application/json"
          disabled={busy || !editor}
          onChange={(event) => {
            void importFile(event.currentTarget.files?.[0])
            event.currentTarget.value = ''
          }}
        />
        {stale && (
          <p role="status" className={styles.description}>
            {t('glyphDatabase.stale')}
          </p>
        )}
        <div className={styles.actions}>
          <Button
            disabled={busy || !dirty || !current}
            onClick={() => void mutate(true)}
          >
            {t('glyphDatabase.save')}
          </Button>
          <Button disabled={busy} onClick={reloadSaved}>
            {t('glyphDatabase.reload')}
          </Button>
        </div>
      </details>
    </details>
  )
}
