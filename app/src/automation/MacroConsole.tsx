import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { PluginManager } from '../plugins/PluginManager'
import { useAutomationStore } from './automationStore'
import { currentMacroContext } from './macroBridge'
import styles from './MacroConsole.module.css'

/** Explicit local-script runner with a stoppable JavaScript console. */
export function MacroConsole() {
  const { t } = useTranslation()
  const state = useAutomationStore()
  const [page, setPage] = useState<'scripts' | 'plugins'>('scripts')
  if (!state.open) return null
  const context = state.running ? state.context : currentMacroContext()
  const busy = state.running || state.reading

  return (
    <Dialog
      title={t('automation.title')}
      size="lg"
      scrollBody
      onCancel={state.close}
      footer={
        <>
          {page === 'scripts' ? (
            <>
              <Button
                disabled={state.running || state.output.length === 0}
                onClick={state.clearOutput}
              >
                {t('automation.clear')}
              </Button>
              {state.running && state.language === 'javascript' ? (
                <Button
                  disabled={state.status === 'stopping'}
                  onClick={state.stop}
                >
                  {t('automation.stop')}
                </Button>
              ) : null}
              <Button
                variant="primary"
                disabled={busy || !state.source.trim()}
                onClick={() => void state.run()}
              >
                {t('automation.run')}
              </Button>
            </>
          ) : null}
          <Button disabled={state.running} onClick={state.close}>
            {t('common.close')}
          </Button>
        </>
      }
    >
      <div
        role="tablist"
        aria-label={t('automation.sections')}
        className={styles.tabs}
      >
        {(['scripts', 'plugins'] as const).map((item) => (
          <Button
            key={item}
            role="tab"
            aria-selected={page === item}
            disabled={state.running}
            onClick={() => setPage(item)}
          >
            {t(`automation.${item}`)}
          </Button>
        ))}
      </div>
      {page === 'plugins' ? (
        <PluginManager />
      ) : (
        <div className={styles.console}>
          <p className={styles.hint}>{t('automation.trust')}</p>
          <div className={styles.folder}>
            <div>
              <strong>{t('automation.folder')}</strong>
              <code className={styles.path}>
                {state.directory || t('automation.folderPending')}
              </code>
            </div>
            <Button
              disabled={state.loading || busy}
              onClick={() => void state.loadScripts(true)}
            >
              {t(state.loading ? 'automation.reloading' : 'automation.reload')}
            </Button>
          </div>
          <div className={styles.fields}>
            <label className={styles.field}>
              <span>{t('automation.script')}</span>
              <select
                value={state.selectedScript}
                disabled={busy || state.loading}
                onChange={(event) => {
                  if (event.target.value)
                    void state.openScript(event.target.value)
                }}
              >
                <option value="">{t('automation.custom')}</option>
                {state.scripts.map((script) => (
                  <option key={script.name} value={script.name}>
                    {script.name}
                  </option>
                ))}
              </select>
            </label>
            <label className={styles.field}>
              <span>{t('automation.language')}</span>
              <select
                value={state.language}
                disabled={busy}
                onChange={(event) =>
                  state.setLanguage(
                    event.target.value as 'javascript' | 'python',
                  )
                }
              >
                <option value="javascript">{t('automation.javascript')}</option>
                <option value="python">{t('automation.python')}</option>
              </select>
            </label>
          </div>
          {state.warnings.length ? (
            <ul className={styles.warnings}>
              {state.warnings.map((warning, index) => (
                <li key={index}>{warning}</li>
              ))}
            </ul>
          ) : null}
          {state.error ? (
            <p role="alert" className={styles.error}>
              {state.error}
            </p>
          ) : null}
          <p className={styles.context}>
            {t('automation.context', {
              doc: context?.doc ?? '—',
              master: context?.master ?? '—',
              glyph: context?.glyph ?? '—',
            })}
          </p>
          <label className={styles.field}>
            <span>{t('automation.source')}</span>
            <textarea
              className={styles.source}
              value={state.source}
              disabled={busy}
              spellCheck={false}
              autoCapitalize="off"
              autoCorrect="off"
              onChange={(event) => state.setSource(event.target.value)}
            />
          </label>
          <p className={styles.hint}>
            {t(
              state.language === 'python'
                ? 'automation.pythonHelp'
                : 'automation.javascriptHelp',
            )}
          </p>
          <div className={styles.outputHeader}>
            <strong>{t('automation.output')}</strong>
            <span role="status">{t(`automation.status.${state.status}`)}</span>
          </div>
          <div
            role="log"
            aria-label={t('automation.output')}
            aria-live="polite"
            className={styles.output}
          >
            {state.output.length ? (
              state.output.map((entry, index) => (
                <pre
                  key={index}
                  className={
                    entry.level === 'error'
                      ? styles.error
                      : entry.level === 'warn'
                        ? styles.warning
                        : undefined
                  }
                >
                  {entry.text}
                </pre>
              ))
            ) : (
              <span className={styles.hint}>{t('automation.emptyOutput')}</span>
            )}
          </div>
        </div>
      )}
    </Dialog>
  )
}
