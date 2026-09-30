import { useEffect, useMemo, useState, type RefObject } from 'react'
import { useTranslation } from 'react-i18next'
import {
  filterProofLibrary,
  proofLibraryGroups,
  stepProofLibrary,
  type ProofEntry,
} from './proofLibrary'
import { useProofLibrary } from './useProofLibrary'
import { ProofExportControls } from './ProofExportControls'
import type { TextRequestContext } from './textLayout'
import styles from './ProofLibraryPanel.module.css'

/** Project-backed proof phrases; arrow stepping only consumes keys in the text input. */
export function ProofLibraryPanel({
  context,
  title,
  inputRef,
  onSelect,
}: {
  context: TextRequestContext
  title: string
  inputRef: RefObject<HTMLInputElement | null>
  onSelect: (entry: ProofEntry) => void
}) {
  const { t } = useTranslation()
  const library = useProofLibrary(context.doc)
  const [selected, setSelected] = useState('')
  const [name, setName] = useState('')
  const [group, setGroup] = useState('')
  const [filterGroup, setFilterGroup] = useState('')
  const [filterScript, setFilterScript] = useState('')
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
  const choose = (entry: ProofEntry) => {
    setSelected(entry.id)
    setName(entry.name)
    setGroup(entry.group)
    onSelect(entry)
  }
  useEffect(() => {
    const input = inputRef.current
    const key = (event: KeyboardEvent) => {
      if (
        event.isComposing ||
        event.altKey ||
        event.ctrlKey ||
        event.metaKey ||
        event.shiftKey ||
        (event.key !== 'ArrowUp' && event.key !== 'ArrowDown')
      )
        return
      const entry = stepProofLibrary(
        entries,
        selected,
        event.key === 'ArrowUp' ? -1 : 1,
      )
      if (!entry) return
      event.preventDefault()
      setSelected(entry.id)
      setName(entry.name)
      setGroup(entry.group)
      onSelect(entry)
    }
    input?.addEventListener('keydown', key)
    return () => input?.removeEventListener('keydown', key)
  }, [entries, inputRef, onSelect, selected])
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
    if (await library.save(entry)) setSelected(entry.id)
  }
  const step = (direction: -1 | 1) => {
    const entry = stepProofLibrary(entries, selected, direction)
    if (entry) choose(entry)
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
              if (current)
                void library.remove(current.id).then((removed) => {
                  if (removed) setSelected('')
                })
            }}
          >
            {t('proof.library.remove')}
          </button>
        </div>
        <p className={styles.hint}>
          {t(
            library.entries.length
              ? 'proof.library.stepping'
              : 'proof.library.empty',
          )}
        </p>
        {library.error && <p role="alert">{library.error}</p>}
        <ProofExportControls
          context={context}
          entries={entries}
          title={title}
        />
      </div>
    </details>
  )
}
