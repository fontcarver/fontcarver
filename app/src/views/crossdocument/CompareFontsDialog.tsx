import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type DocId,
  type DocSummary,
  type FontCompareFamilyResult,
  type FontCompareResult,
  type FontCompareGlyphResult,
} from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { withSettledDocument } from '../../editor/documentPersistence'
import { errorNotice } from '../../i18n/errors'
import { documentLabel, useSessionStore } from '../../stores/sessionStore'
import { ComparisonResults } from './ComparisonResults'
import { ComparisonOutline } from './ComparisonOutline'
import { DocumentMasterFields } from './DocumentMasterFields'
import { FamilyResults } from './FamilyResults'
import { MergeControls } from './MergeControls'
import { useFontMerge } from './useFontMerge'
import styles from './crossDocument.module.css'

/** Font differences, reference binaries, family checks and conflict-resolved merge. */
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
  const [references, setReferences] = useState<Map<DocId, string>>(new Map())
  const [differencesOnly, setDifferencesOnly] = useState(true)
  const [allMasters, setAllMasters] = useState(true)
  const [result, setResult] = useState<FontCompareResult | null>(null)
  const [outline, setOutline] = useState<FontCompareGlyphResult | null>(null)
  const [family, setFamily] = useState<FontCompareFamilyResult | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const failure = (reason: unknown) => {
    const message = errorNotice(reason, t)
    setError([message.message, message.details].filter(Boolean).join(' · '))
  }
  const session = useFontMerge({
    doc: target.doc,
    sourceDoc,
    targetMaster,
    sourceMaster,
    result,
    onFailure: failure,
    onBusy: setBusy,
  })
  const invalidate = () => {
    setResult(null)
    setOutline(null)
    setFamily(null)
    setError(null)
    setNotice(null)
    session.reset()
  }
  const compare = async (offset = 0) => {
    if (sourceDoc === null) return
    setBusy(true)
    setError(null)
    setOutline(null)
    session.reset()
    try {
      setResult(
        await withSettledDocument(sourceDoc, () =>
          withSettledDocument(target.doc, () =>
            call('font.compare', {
              doc: target.doc,
              source_doc: sourceDoc,
              master: targetMaster || null,
              source_master: sourceMaster || null,
              offset,
              limit: 100,
              differing_only: differencesOnly,
            }),
          ),
        ),
      )
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
    setFamily(null)
    try {
      setFamily(
        await call('font.compare_family', {
          doc: target.doc,
          master: targetMaster || null,
          docs: docs
            .filter((doc) => doc.doc !== target.doc)
            .map((doc) => doc.doc),
          all_masters: allMasters,
        }),
      )
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
      const reference = await call('font.open_reference', {
        path: referencePath.trim(),
        face_index: null,
      })
      // `refreshSummary` only refreshes documents the store already knows, so a
      // freshly opened reference is registered here before it can be selected.
      const summary = await call('doc.summary', { doc: reference.doc })
      useSessionStore.setState((state) =>
        state.docs.some((entry) => entry.doc === summary.doc)
          ? state
          : { docs: [...state.docs, summary] },
      )
      setReferences((current) =>
        new Map(current).set(reference.doc, reference.path),
      )
      setSourceDoc(reference.doc)
      setSourceMaster(reference.masters[0]?.id ?? '')
      setReferencePath('')
      setNotice(
        t('crossDocument.referenceOpened', {
          family: reference.family,
          path: reference.path,
        }),
      )
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
  const applyMerge = async () => {
    const applied = await session.apply()
    if (!applied) return
    setNotice(
      t('crossDocument.merged', {
        count: applied.replaced.length,
        label: applied.label,
      }),
    )
    setResult(null)
    setOutline(null)
    setFamily(null)
    await useSessionStore.getState().refreshSummary(target.doc)
  }
  const readOnlySource =
    sourceDoc === null ? undefined : references.get(sourceDoc)
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
            disabled={busy || !result || session.empty}
            onClick={() => {
              void session.preview()
            }}
          >
            {t('crossDocument.previewMerge')}
          </Button>
          <Button
            variant="primary"
            disabled={busy || !session.merge}
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
        {readOnlySource !== undefined && (
          <p role="status" className={styles.help}>
            {t('crossDocument.readOnlySource', { path: readOnlySource })}
          </p>
        )}
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
        <p className={styles.help}>{t('crossDocument.referenceHelp')}</p>
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
          <label>
            <input
              type="checkbox"
              checked={allMasters}
              disabled={busy}
              onChange={(event) => setAllMasters(event.target.checked)}
            />{' '}
            {t('crossDocument.allMasters')}
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
            disabled={busy}
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
        {family && (
          <FamilyResults
            family={family}
            labelFor={(doc) => {
              const summary = docs.find((item) => item.doc === doc)
              return summary ? documentLabel(summary) : String(doc)
            }}
          />
        )}
        <ComparisonResults
          result={result}
          busy={busy}
          choices={session.choices}
          onChoice={session.resolve}
          onInspect={(name) => {
            void inspect(name)
          }}
          onPage={(offset) => {
            void compare(offset)
          }}
        />
        {outline && <ComparisonOutline outline={outline} />}
        {result && (
          <MergeControls
            adopt={session.adopt}
            disabled={busy}
            onAdopt={session.setAdopt}
            merge={session.merge?.result ?? null}
            missingBases={session.missingBases}
            onMissingBases={session.setMissingBases}
          />
        )}
      </div>
    </Dialog>
  )
}
