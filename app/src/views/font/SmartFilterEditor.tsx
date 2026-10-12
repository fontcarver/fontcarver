import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call, type SavedSmartFilter, type SmartFilterNode } from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'

import { SmartFilterBuilder } from './SmartFilterBuilder'
import { defaultSmartRule, formatSmartFilter } from './smartFilterBuilderModel'
import styles from './smartFilters.module.css'

export interface SmartFilterEditorProps {
  initialFilter?: SavedSmartFilter
  onSave: (filter: SavedSmartFilter) => Promise<void>
  onCancel: () => void
}

const INITIAL_ROOT: SmartFilterNode = {
  kind: 'all',
  children: [defaultSmartRule('export')],
}

/** Edits a saved predicate with synchronized visual rules and a custom DSL. */
export function SmartFilterEditor({
  initialFilter,
  onSave,
  onCancel,
}: SmartFilterEditorProps) {
  const { t } = useTranslation()
  const [name, setName] = useState(initialFilter?.name ?? '')
  const [expression, setExpression] = useState(
    initialFilter?.expression ?? formatSmartFilter(INITIAL_ROOT),
  )
  const [root, setRoot] = useState<SmartFilterNode>(INITIAL_ROOT)
  const [mode, setMode] = useState<'builder' | 'expression'>(
    initialFilter ? 'expression' : 'builder',
  )
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const showBuilder = async () => {
    if (busy || mode === 'builder') return
    setBusy(true)
    setError(null)
    try {
      const parsed = await call('glyph.smart_filters.parse', { expression })
      setRoot(
        parsed.root.kind === 'predicate'
          ? { kind: 'all', children: [parsed.root] }
          : parsed.root,
      )
      setExpression(parsed.expression)
      setMode('builder')
    } catch (cause) {
      setError(
        cause instanceof Error ? cause.message : t('smartFilters.parseError'),
      )
    } finally {
      setBusy(false)
    }
  }

  const save = async () => {
    if (busy || !name.trim() || !expression.trim()) return
    setBusy(true)
    setError(null)
    try {
      const parsed = await call('glyph.smart_filters.parse', { expression })
      await onSave({ name: name.trim(), expression: parsed.expression })
    } catch (cause) {
      setError(
        cause instanceof Error ? cause.message : t('smartFilters.saveError'),
      )
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog
      title={t(
        initialFilter ? 'smartFilters.editTitle' : 'smartFilters.newTitle',
      )}
      size="lg"
      scrollBody
      onCancel={() => {
        if (!busy) onCancel()
      }}
      onConfirm={() => void save()}
      footer={
        <>
          <Button disabled={busy} onClick={onCancel}>
            {t('smartFilters.cancel')}
          </Button>
          <Button
            variant="primary"
            disabled={busy || !name.trim() || !expression.trim()}
            onClick={() => void save()}
          >
            {t(busy ? 'smartFilters.saving' : 'smartFilters.save')}
          </Button>
        </>
      }
    >
      <div className={styles.editor}>
        <label className={styles.label}>
          {t('smartFilters.name')}
          <input
            className={styles.control}
            value={name}
            maxLength={128}
            disabled={busy}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        <div className={styles.mode}>
          <Button
            aria-pressed={mode === 'builder'}
            disabled={busy}
            onClick={() => void showBuilder()}
          >
            {t('smartFilters.builder')}
          </Button>
          <Button
            aria-pressed={mode === 'expression'}
            disabled={busy}
            onClick={() => setMode('expression')}
          >
            {t('smartFilters.custom')}
          </Button>
        </div>
        {mode === 'builder' ? (
          <SmartFilterBuilder
            root={root}
            disabled={busy}
            onChange={(next) => {
              setRoot(next)
              setExpression(formatSmartFilter(next))
              setError(null)
            }}
          />
        ) : (
          <label className={styles.label}>
            {t('smartFilters.expression')}
            <textarea
              className={styles.expression}
              value={expression}
              rows={5}
              disabled={busy}
              spellCheck={false}
              onChange={(event) => {
                setExpression(event.target.value)
                setError(null)
              }}
            />
          </label>
        )}
        <p className={styles.help}>{t('smartFilters.expressionHelp')}</p>
        {error && (
          <p role="alert" className={styles.error}>
            {error}
          </p>
        )}
      </div>
    </Dialog>
  )
}
