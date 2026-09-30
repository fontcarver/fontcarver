import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type DocId,
  type DocSummary,
  type FontCompareResult,
  type FontCompareGlyphResult,
  type PasteSpecialParams,
  type PasteSpecialResult,
} from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { withSettledDocument } from '../../editor/documentPersistence'
import { errorNotice } from '../../i18n/errors'
import { useSessionStore } from '../../stores/sessionStore'
import { ComparisonResults } from './ComparisonResults'
import { ComparisonOutline } from './ComparisonOutline'
import { DocumentMasterFields } from './DocumentMasterFields'
import styles from './crossDocument.module.css'

/** Font differences, reference-binary overlays, family checks and reviewed merge. */
export function CompareFontsDialog({
  target,
  onClose,
}: {
  target: DocSummary
  onClose: () => void
}) {
  const { t } = useTranslation()
  const docs = useSessionStore((state) => state.docs)
  const initialSource = docs.find((doc) => doc.doc !== target.doc)
  const [sourceDoc, setSourceDoc] = useState<DocId | null>(
    initialSource?.doc ?? null,
  )
  const [sourceMaster, setSourceMaster] = useState(
    initialSource?.masters[0]?.id ?? '',
  )
  const [targetMaster, setTargetMaster] = useState(target.masters[0]?.id ?? '')
  const [referencePath, setReferencePath] = useState('')
  const [differencesOnly, setDifferencesOnly] = useState(true)
  const [result, setResult] = useState<FontCompareResult | null>(null)
  const [outline, setOutline] = useState<FontCompareGlyphResult | null>(null)
  const [family, setFamily] = useState<
    { doc: DocSummary; result: FontCompareResult }[]
  >([])
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [merge, setMerge] = useState<{
    params: PasteSpecialParams
    result: PasteSpecialResult
  } | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const invalidate = () => {
    setResult(null)
    setOutline(null)
    setFamily([])
    setSelected(new Set())
    setMerge(null)
    setError(null)
    setNotice(null)
  }
  const failure = (reason: unknown) => {
    const message = errorNotice(reason, t)
    setError([message.message, message.details].filter(Boolean).join(' · '))
  }
  const query = (doc: DocId, master: string, offset: number, limit = 100) =>
    withSettledDocument(doc, () =>
      withSettledDocument(target.doc, () =>
        call('font.compare', {
          doc: target.doc,
          source_doc: doc,
          master: targetMaster || null,
          source_master: master || null,
          offset,
          limit,
          differing_only: differencesOnly,
        }),
      ),
    )
  const compare = async (offset = 0) => {
    if (sourceDoc === null) return
    setBusy(true)
    setError(null)
    setMerge(null)
    setOutline(null)
    setSelected(new Set())
    try {
      setResult(await query(sourceDoc, sourceMaster, offset))
    } catch (reason) {
      setResult(null)
      failure(reason)
    } finally {
      setBusy(false)
    }
  }
  const compareFamily = async () => {
    setBusy(true)
    setError(null)
    setFamily([])
    try {
      const rows: { doc: DocSummary; result: FontCompareResult }[] = []
      for (const doc of docs.filter((doc) => doc.doc !== target.doc))
        rows.push({
          doc,
          result: await query(doc.doc, doc.masters[0]?.id ?? '', 0, 1),
        })
      setFamily(rows)
    } catch (reason) {
      failure(reason)
    } finally {
      setBusy(false)
    }
  }
  const openReference = async () => {
    if (!referencePath.trim()) return
    setBusy(true)
    invalidate()
    try {
      const summary = await useSessionStore
        .getState()
        .open(referencePath.trim())
      if (summary) {
        setSourceDoc(summary.doc)
        setSourceMaster(summary.masters[0]?.id ?? '')
        setReferencePath('')
      }
    } catch (reason) {
      failure(reason)
    } finally {
      setBusy(false)
    }
  }
  const inspect = async (name: string) => {
    if (sourceDoc === null) return
    setBusy(true)
    setError(null)
    setOutline(null)
    try {
      setOutline(
        await call('font.compare_glyph', {
          doc: target.doc,
          source_doc: sourceDoc,
          master: targetMaster || null,
          source_master: sourceMaster || null,
          name,
          expected_generation: result?.generation ?? null,
          expected_source_generation: result?.source_generation ?? null,
        }),
      )
    } catch (reason) {
      failure(reason)
    } finally {
      setBusy(false)
    }
  }
  const previewMerge = async () => {
    if (!result || sourceDoc === null) return
    setBusy(true)
    setError(null)
    setNotice(null)
    const params: PasteSpecialParams = {
      doc: target.doc,
      source_doc: sourceDoc,
      mappings: result.glyphs
        .filter((glyph) => selected.has(glyph.name) && glyph.source !== null)
        .map((glyph) => ({ source: glyph.source!, target: null })),
      destination: 'same_name',
      content: 'all_data',
      source_master: sourceMaster || null,
      target_master: targetMaster || null,
      missing_bases: 'copy',
      dry_run: true,
      expected_generation: result.generation,
      expected_source_generation: result.source_generation,
    }
    try {
      setMerge({ params, result: await call('doc.paste_special', params) })
    } catch (reason) {
      setMerge(null)
      failure(reason)
    } finally {
      setBusy(false)
    }
  }
  const applyMerge = async () => {
    if (!merge) return
    setBusy(true)
    setError(null)
    try {
      await withSettledDocument(merge.params.source_doc, () =>
        withSettledDocument(target.doc, () =>
          call('doc.paste_special', {
            ...merge.params,
            dry_run: false,
            expected_generation: merge.result.generation,
            expected_source_generation: merge.result.source_generation,
          }),
        ),
      )
      setNotice(
        t('crossDocument.merged', { count: merge.result.pasted.length }),
      )
      setMerge(null)
      setResult(null)
      setOutline(null)
      setFamily([])
      setSelected(new Set())
      await useSessionStore.getState().refreshSummary(target.doc)
    } catch (reason) {
      setMerge(null)
      failure(reason)
    } finally {
      setBusy(false)
    }
  }
  return (
    <Dialog
      title={t('crossDocument.compareTitle')}
      size="lg"
      scrollBody
      onCancel={() => {
        if (!busy) onClose()
      }}
      footer={
        <>
          <Button disabled={busy} onClick={onClose}>
            {t('common.close')}
          </Button>
          <Button
            disabled={busy || selected.size === 0}
            onClick={() => {
              void previewMerge()
            }}
          >
            {t('crossDocument.previewMerge')}
          </Button>
          <Button
            variant="primary"
            disabled={busy || !merge}
            onClick={() => {
              void applyMerge()
            }}
          >
            {t('crossDocument.merge')}
          </Button>
        </>
      }
    >
      <div className={styles.form} aria-busy={busy}>
        <DocumentMasterFields
          docs={docs}
          sourceDoc={sourceDoc}
          sourceMaster={sourceMaster}
          target={target}
          targetMaster={targetMaster}
          disabled={busy}
          onSource={(doc) => {
            setSourceDoc(doc)
            setSourceMaster(
              docs.find((item) => item.doc === doc)?.masters[0]?.id ?? '',
            )
            invalidate()
          }}
          onSourceMaster={(master) => {
            setSourceMaster(master)
            invalidate()
          }}
          onTargetMaster={(master) => {
            setTargetMaster(master)
            invalidate()
          }}
        />
        <div className={styles.controls}>
          <label className={styles.field}>
            {t('crossDocument.referencePath')}
            <input
              disabled={busy}
              value={referencePath}
              onChange={(event) => setReferencePath(event.target.value)}
            />
          </label>
          <Button
            disabled={busy || !referencePath.trim()}
            onClick={() => {
              void openReference()
            }}
          >
            {t('crossDocument.openReference')}
          </Button>
        </div>
        <div className={styles.controls}>
          <label>
            <input
              type="checkbox"
              checked={differencesOnly}
              disabled={busy}
              onChange={(event) => {
                setDifferencesOnly(event.target.checked)
                invalidate()
              }}
            />{' '}
            {t('crossDocument.differencesOnly')}
          </label>
          <Button
            disabled={busy || sourceDoc === null}
            onClick={() => {
              void compare()
            }}
          >
            {t('crossDocument.compare')}
          </Button>
          <Button
            disabled={busy || docs.length < 2}
            onClick={() => {
              void compareFamily()
            }}
          >
            {t('crossDocument.compareFamily')}
          </Button>
        </div>
        {error && (
          <p role="alert" className={styles.error}>
            {error}
          </p>
        )}
        {notice && <p role="status">{notice}</p>}
        <ComparisonResults
          family={family}
          result={result}
          busy={busy}
          selected={selected}
          onSelected={(names) => {
            setSelected(names)
            setMerge(null)
          }}
          onInspect={(name) => {
            void inspect(name)
          }}
          onPage={(offset) => {
            void compare(offset)
          }}
        />
        {outline && <ComparisonOutline outline={outline} />}
        {merge && (
          <section aria-label={t('crossDocument.review')}>
            <p role="status">
              {t('crossDocument.mergeReview', {
                names: merge.result.pasted.join(', '),
                bases: merge.result.added_bases.length,
              })}
            </p>
            {merge.result.warnings.map((warning, index) => (
              <p key={index} role="note">
                {warning}
              </p>
            ))}
          </section>
        )}
      </div>
    </Dialog>
  )
}
