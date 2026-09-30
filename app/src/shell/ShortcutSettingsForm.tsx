import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { useKeymapStore } from '../stores/keymapStore'
import { keymapConflicts, shortcutFor, type KeymapOverrides } from './keymap'
import { parseKeymapDocument, serializeKeymap } from './keymapDocument'
import { KEYMAP_PRESETS } from './keymapPresets'
import {
  FIXED_SHORTCUT_IDS,
  keymapTargets,
  reservedKeymapTargets,
} from './keymapTargets'
import { formatShortcut } from './shortcuts'
import { helpHint } from './contextHelp'
import { useCommandRegistry } from './useCommandRegistry'
import styles from './ShortcutSettingsDialog.module.css'

/** Reviewed draft for explicit shortcut changes. */
export function ShortcutSettingsForm() {
  const { t } = useTranslation()
  useCommandRegistry()
  const [draft, setDraft] = useState<KeymapOverrides>(() => ({
    ...useKeymapStore.getState().overrides,
  }))
  const [query, setQuery] = useState('')
  const [preset, setPreset] = useState('default')
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const importVersion = useRef(0)
  const targets = keymapTargets()
  const allTargets = [...targets, ...reservedKeymapTargets()]
  const conflicts = keymapConflicts(allTargets, draft)
  const known = new Set(targets.map((target) => target.id))
  const unknown = Object.keys(draft).filter((id) => !known.has(id))
  let valid = true
  try {
    parseKeymapDocument(serializeKeymap(draft))
  } catch {
    valid = false
  }
  const filtered = targets.filter((target) =>
    `${t(target.title)} ${target.id} ${shortcutFor(target.id, target.shortcut, draft) ?? ''}`
      .toLowerCase()
      .includes(query.toLowerCase()),
  )
  const close = () => {
    importVersion.current += 1
    useKeymapStore.getState().setOpen(false)
  }
  const setBinding = (id: string, value: string | null) => {
    setError('')
    setNotice('')
    setDraft((old) => ({ ...old, [id]: value || null }))
  }
  const restore = (id: string) =>
    setDraft((old) => {
      const next = { ...old }
      delete next[id]
      return next
    })
  const apply = () => {
    if (!valid || conflicts.length > 0 || unknown.length > 0) return
    if (useKeymapStore.getState().save(draft)) close()
    else setError('shortcuts.error.storage')
  }
  const importFile = async (file: File | undefined) => {
    if (!file) return
    const version = ++importVersion.current
    try {
      if (file.size > 262_144) throw new Error('shortcuts.error.tooLarge')
      const imported = parseKeymapDocument(await file.text())
      if (version !== importVersion.current) return
      setDraft(imported)
      setError('')
      setNotice('shortcuts.imported')
    } catch (cause) {
      if (version !== importVersion.current) return
      setError(
        cause instanceof Error && cause.message.startsWith('shortcuts.')
          ? cause.message
          : 'shortcuts.error.read',
      )
    }
  }
  const exportFile = () => {
    const blob = new Blob([serializeKeymap(draft)], {
      type: 'application/json',
    })
    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = 'fontcarver-keymap.json'
    link.click()
    setTimeout(() => URL.revokeObjectURL(url), 0)
  }

  return (
    <Dialog
      title={t('cmd.help.shortcuts')}
      size="lg"
      scrollBody
      onCancel={close}
      footer={
        <>
          <Button onClick={close}>{t('common.cancel')}</Button>
          <Button
            variant="primary"
            onClick={apply}
            disabled={!valid || conflicts.length > 0 || unknown.length > 0}
          >
            {t('shortcuts.apply')}
          </Button>
        </>
      }
    >
      <p className={styles.hint}>{t('shortcuts.description')}</p>
      <div className={styles.controls} {...helpHint('shortcuts')}>
        <label>
          {t('shortcuts.preset.label')}
          <select
            value={preset}
            onChange={(event) => setPreset(event.target.value)}
          >
            <option value="default">{t('shortcuts.preset.default')}</option>
            {KEYMAP_PRESETS.map((item) => (
              <option key={item.id} value={item.id}>
                {t(item.nameKey)}
              </option>
            ))}
          </select>
        </label>
        <Button
          title={t('shortcuts.preset.hint')}
          onClick={() => {
            importVersion.current += 1
            setDraft({
              ...(KEYMAP_PRESETS.find((item) => item.id === preset)
                ?.overrides ?? {}),
            })
            setError('')
            setNotice('shortcuts.preset.loaded')
          }}
        >
          {t('shortcuts.preset.load')}
        </Button>
        <Button
          onClick={exportFile}
          disabled={!valid || conflicts.length > 0 || unknown.length > 0}
        >
          {t('shortcuts.export')}
        </Button>
      </div>
      <label className={styles.import}>
        {t('shortcuts.import')}
        <input
          type="file"
          accept=".json,application/json"
          onChange={(event) => {
            void importFile(event.target.files?.[0])
            event.target.value = ''
          }}
        />
      </label>
      {notice && (
        <p role="status" className={styles.hint}>
          {t(notice)}
        </p>
      )}
      <p className={styles.hint}>{t('shortcuts.fixedHint')}</p>
      {(error || !valid || conflicts.length > 0 || unknown.length > 0) && (
        <div role="alert" className={styles.alert}>
          {error && <p>{t(error)}</p>}
          {!valid && <p>{t('shortcuts.error.binding')}</p>}
          {unknown.length > 0 && (
            <p>
              {t('shortcuts.error.unknown', { ids: unknown.join(', ') })}{' '}
              <Button
                onClick={() =>
                  setDraft((old) =>
                    Object.fromEntries(
                      Object.entries(old).filter(
                        ([id]) => known.has(id) && !FIXED_SHORTCUT_IDS.has(id),
                      ),
                    ),
                  )
                }
              >
                {t('shortcuts.removeUnknown')}
              </Button>
            </p>
          )}
          {conflicts.map((conflict) => (
            <p key={conflict.shortcut}>
              {t('shortcuts.conflict', {
                shortcut: formatShortcut(conflict.shortcut),
                commands: conflict.ids
                  .map((id) =>
                    t(
                      allTargets.find((target) => target.id === id)?.title ??
                        id,
                    ),
                  )
                  .join(', '),
              })}
            </p>
          ))}
        </div>
      )}
      <label className={styles.search}>
        {t('shortcuts.search')}
        <input
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder={t('shortcuts.searchHint')}
        />
      </label>
      <div
        className={styles.bindings}
        role="group"
        aria-label={t('shortcuts.bindings')}
      >
        {filtered.slice(0, 80).map((target) => (
          <div className={styles.row} key={target.id}>
            <label htmlFor={`binding-${target.id}`}>
              <span>{t(target.title)}</span>
              <small>{t(`shortcuts.scope.${target.scope}`)}</small>
            </label>
            <input
              id={`binding-${target.id}`}
              aria-label={t('shortcuts.bindingLabel', {
                command: t(target.title),
              })}
              value={shortcutFor(target.id, target.shortcut, draft) ?? ''}
              placeholder={t('shortcuts.unbound')}
              spellCheck={false}
              onChange={(event) => setBinding(target.id, event.target.value)}
            />
            <Button
              aria-label={t('shortcuts.restoreLabel', {
                command: t(target.title),
              })}
              title={t('shortcuts.restoreHint')}
              onClick={() => restore(target.id)}
            >
              {t('shortcuts.restore')}
            </Button>
          </div>
        ))}
        {filtered.length === 0 && (
          <p className={styles.hint}>{t('shortcuts.empty')}</p>
        )}
      </div>
      {filtered.length > 80 && (
        <p className={styles.hint}>
          {t('shortcuts.more', { count: filtered.length })}
        </p>
      )}
    </Dialog>
  )
}
