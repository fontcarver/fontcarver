import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type RefObject,
} from 'react'
import { useTranslation } from 'react-i18next'
import { reportError } from '../../shell/commands'
import { saveArtifact } from '../../shell/saveArtifact'
import { exportProofText, readProofFile } from './proofInterchange'
import {
  filterProofLibrary,
  proofLibraryGroups,
  stepProofLibrary,
  type ProofEntry,
} from './proofLibrary'
import { builtinProofEntries } from './proofSamples'
import { proofStepDirection } from './proofStepping'
import { useProofLibrary } from './useProofLibrary'
import { ProofExportControls } from './ProofExportControls'
import type { TextRequestContext } from './textLayout'
import styles from './ProofLibraryPanel.module.css'

function fileName(title: string): string {
  return `${title.replace(/[^\p{L}\p{N}._ -]/gu, '_') || 'proofs'}-proofs.txt`
}

/** Project-backed proof phrases; arrow stepping only consumes keys in the text input. */
export function ProofLibraryPanel({
  context,
  title,
  inputRef,
  selectedId,
  stepRef,
  onSelect,
}: {
  context: TextRequestContext
  title: string
  inputRef: RefObject<HTMLInputElement | null>
  /** The entry the tab last applied, so stepping continues from it. */
  selectedId?: string | null
  /** Receives a stepper the text view calls while the canvas has focus. */
  stepRef?: RefObject<((direction: -1 | 1) => void) | null>
  onSelect: (entry: ProofEntry) => void
}) {
  const { t } = useTranslation()
  const library = useProofLibrary(context.doc)
  const selected = selectedId ?? ''
  const [name, setName] = useState('')
  const [group, setGroup] = useState('')
  const [filterGroup, setFilterGroup] = useState('')
  const [filterScript, setFilterScript] = useState('')
  const [importRejected, setImportRejected] = useState(false)
  const fileInput = useRef<HTMLInputElement>(null)
  const groups = useMemo(
    () => proofLibraryGroups(library.entries),
    [library.entries],
  )
  const scripts = useMemo(
    () =>
      [
        ...new Set(
          library.entries.flatMap((entry) =>
            entry.script ? [entry.script] : [],
          ),
        ),
      ].sort(),
    [library.entries],
  )
  const entries = useMemo(
    () =>
      filterProofLibrary(library.entries, {
        group: filterGroup || undefined,
        script: filterScript || undefined,
      }),
    [library.entries, filterGroup, filterScript],
  )
  const current = library.entries.find((entry) => entry.id === selected)
  // The name and group fields follow the selection without fighting an edit in
  // progress: they are reset only when a different entry becomes current.
  const [shownId, setShownId] = useState('')
  if (current && shownId !== current.id) {
    setShownId(current.id)
    setName(current.name)
    setGroup(current.group)
  }
  const choose = useCallback(
    (entry: ProofEntry) => {
      setImportRejected(false)
      onSelect(entry)
    },
    [onSelect],
  )
  // The stepper's inputs are named as dependencies rather than captured by a
  // closure that is rebuilt on every render: automatic memoisation keeps a
  // closure whose inputs it cannot see, which would step from a stale
  // selection.
  const step = useCallback(
    (direction: -1 | 1) => {
      const entry = stepProofLibrary(entries, selected || null, direction)
      if (entry) choose(entry)
    },
    [entries, selected, choose],
  )
  // The two keyboard paths are installed once and always call the newest
  // stepper, so a filter change takes effect without reinstalling listeners.
  const stepLatest = useRef(step)
  useEffect(() => {
    stepLatest.current = step
  }, [step])
  useEffect(() => {
    const input = inputRef.current
    const key = (event: KeyboardEvent) => {
      const direction = proofStepDirection(event)
      if (direction === null) return
      event.preventDefault()
      stepLatest.current(direction)
    }
    input?.addEventListener('keydown', key)
    return () => input?.removeEventListener('keydown', key)
  }, [inputRef])
  useEffect(() => {
    if (!stepRef) return
    const stepper = (direction: -1 | 1) => stepLatest.current(direction)
    stepRef.current = stepper
    return () => {
      if (stepRef.current === stepper) stepRef.current = null
    }
  }, [stepRef])
  const save = async (replace: boolean) => {
    const entry: ProofEntry = {
      id: replace && current ? current.id : crypto.randomUUID(),
      name: name.trim() || Array.from(context.text).slice(0, 80).join(''),
      group: group.trim(),
      text: context.text,
      script: context.script ?? '',
      language: context.language ?? '',
      direction:
        context.direction === 'rtl' || context.direction === 'ttb'
          ? context.direction
          : 'ltr',
    }
    if (await library.save(entry)) onSelect(entry)
  }
  const exportText = async () => {
    setImportRejected(false)
    await saveArtifact(
      fileName(title),
      new TextEncoder().encode(exportProofText(library.entries)),
      'text/plain;charset=utf-8',
      t('proof.library.interchangeFilter'),
    )
  }
  const importFile = async (file: File) => {
    setImportRejected(false)
    const text = await readProofFile(file)
    // The hook reports why; this only says that the stored library is unchanged.
    setImportRejected(!(await library.importText(text)))
  }
  return (
    <details className={styles.panel}>
      <summary>
        {t('proof.library.title')}{' '}
        <span>
          {t('proof.library.count', { count: library.entries.length })}
        </span>
      </summary>
      <div className={styles.body}>
        <div className={styles.row}>
          <label>
            {t('proof.library.group')}
            <select
              value={filterGroup}
              onChange={(event) => setFilterGroup(event.currentTarget.value)}
              aria-label={t('proof.library.allGroups')}
            >
              <option value="">{t('proof.library.allGroups')}</option>
              {groups.map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </select>
          </label>
          <label>
            {t('proof.library.filterScript')}
            <select
              value={filterScript}
              onChange={(event) => setFilterScript(event.currentTarget.value)}
            >
              <option value="">{t('proof.library.allScripts')}</option>
              {scripts.map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </select>
          </label>
          <select
            className={styles.entrySelect}
            aria-label={t('proof.library.choose')}
            value={
              entries.some((entry) => entry.id === selected) ? selected : ''
            }
            onChange={(event) => {
              const entry = entries.find(
                (candidate) => candidate.id === event.currentTarget.value,
              )
              if (entry) choose(entry)
            }}
            disabled={library.loading || !entries.length}
          >
            <option value="">{t('proof.library.choose')}</option>
            {entries.map((entry) => (
              <option key={entry.id} value={entry.id}>
                {entry.group ? `${entry.group} · ` : ''}
                {entry.name}
              </option>
            ))}
          </select>
          <button
            type="button"
            onClick={() => step(-1)}
            disabled={!entries.length}
            aria-label={t('proof.library.previous')}
          >
            ↑
          </button>
          <button
            type="button"
            onClick={() => step(1)}
            disabled={!entries.length}
            aria-label={t('proof.library.next')}
          >
            ↓
          </button>
        </div>
        <div className={styles.row}>
          <label>
            {t('proof.library.name')}
            <input
              value={name}
              maxLength={128}
              onChange={(event) => setName(event.currentTarget.value)}
            />
          </label>
          <label>
            {t('proof.library.group')}
            <input
              value={group}
              maxLength={128}
              onChange={(event) => setGroup(event.currentTarget.value)}
            />
          </label>
          <button
            type="button"
            disabled={library.saving || library.loading || !context.text.trim()}
            onClick={() => void save(false)}
          >
            {t('proof.library.save')}
          </button>
          <button
            type="button"
            disabled={library.saving || !current || !context.text.trim()}
            onClick={() => void save(true)}
          >
            {t('proof.library.update')}
          </button>
          <button
            type="button"
            disabled={library.saving || !current}
            onClick={() => {
              if (current) void library.remove(current.id).catch(reportError)
            }}
          >
            {t('proof.library.remove')}
          </button>
        </div>
        <div className={styles.row}>
          <button
            type="button"
            disabled={library.saving || library.loading}
            onClick={() => {
              setImportRejected(false)
              void library.adopt(builtinProofEntries(t)).catch(reportError)
            }}
          >
            {t('proof.library.adoptSamples')}
          </button>
          <button
            type="button"
            disabled={library.saving || !library.entries.length}
            onClick={() => void exportText().catch(reportError)}
          >
            {t('proof.library.exportText')}
          </button>
          <button
            type="button"
            disabled={library.saving || library.loading}
            onClick={() => fileInput.current?.click()}
          >
            {t('proof.library.importText')}
          </button>
          <input
            ref={fileInput}
            className={styles.fileInput}
            type="file"
            accept=".txt,.tsv,text/plain,text/tab-separated-values"
            tabIndex={-1}
            aria-hidden="true"
            onChange={(event) => {
              const file = event.currentTarget.files?.[0]
              event.currentTarget.value = ''
              if (file) void importFile(file).catch(reportError)
            }}
          />
        </div>
        <p className={styles.hint}>
          {t(
            library.entries.length
              ? 'proof.library.stepping'
              : 'proof.library.empty',
          )}
        </p>
        {(library.error || importRejected) && (
          <p role="alert">
            {[
              importRejected ? t('proof.library.importFailed') : '',
              library.error,
            ]
              .filter(Boolean)
              .join(' ')}
          </p>
        )}
        <ProofExportControls
          context={context}
          entries={entries}
          title={title}
        />
      </div>
    </details>
  )
}
