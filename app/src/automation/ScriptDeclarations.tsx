import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  SCRIPTING_DECLARATIONS,
  SCRIPTING_DECLARATIONS_FILE,
} from '../api/generated/scriptingManifest'
import { Button } from '../components/Button'
import { useAutomationStore } from './automationStore'
import { callSnippet, searchScriptMethods } from './scriptCheck'
import styles from './MacroConsole.module.css'

/** The shipped scripting contract: declarations plus method completion. */
export function ScriptDeclarations() {
  const { t } = useTranslation()
  const [query, setQuery] = useState('')
  const [copied, setCopied] = useState(false)
  const insertSnippet = useAutomationStore((state) => state.insertSnippet)
  const matches = useMemo(() => searchScriptMethods(query), [query])
  const copy = async () => {
    setCopied(false)
    try {
      await globalThis.navigator?.clipboard?.writeText(SCRIPTING_DECLARATIONS)
      setCopied(true)
    } catch {
      // A blocked clipboard leaves the visible declaration text usable.
    }
  }
  return (
    <section className={styles.console} aria-label={t('automation.api')}>
      <p className={styles.hint}>{t('automation.declarationsHelp')}</p>
      <div className={styles.folder}>
        <div>
          <strong>{t('automation.declarationsFile')}</strong>
          <code className={styles.path}>{SCRIPTING_DECLARATIONS_FILE}</code>
        </div>
        <Button onClick={() => void copy()}>
          {t('automation.copyDeclarations')}
        </Button>
      </div>
      {copied ? (
        <p role="status" className={styles.hint}>
          {t('automation.copied')}
        </p>
      ) : null}
      <label className={styles.field}>
        <span>{t('automation.declarations')}</span>
        <textarea
          className={styles.source}
          readOnly
          spellCheck={false}
          value={SCRIPTING_DECLARATIONS}
        />
      </label>
      <label className={styles.field}>
        <span>{t('automation.methodSearch')}</span>
        <input
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      </label>
      <table className={styles.methods}>
        <thead>
          <tr>
            <th>{t('automation.method')}</th>
            <th>{t('automation.parameters')}</th>
            <th>{t('automation.insert')}</th>
          </tr>
        </thead>
        <tbody>
          {matches.map((method) => (
            <tr key={method.method}>
              <td>
                <code>{method.method}</code>
                <span className={styles.path}>
                  {method.params} → {method.result}
                </span>
              </td>
              <td>
                {method.fields.length === 0
                  ? t('automation.noParameters')
                  : method.fields
                      .map(
                        (field) =>
                          `${field.name}${field.optional ? '?' : ''}: ${field.type}`,
                      )
                      .join(', ')}
              </td>
              <td>
                <Button
                  aria-label={t('automation.insertNamed', {
                    method: method.method,
                  })}
                  onClick={() => insertSnippet(callSnippet(method))}
                >
                  {t('automation.insert')}
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {matches.length === 0 ? (
        <p role="status" className={styles.hint}>
          {t('automation.noMethods')}
        </p>
      ) : null}
    </section>
  )
}
