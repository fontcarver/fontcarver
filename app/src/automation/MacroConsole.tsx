import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { PluginManager } from '../plugins/PluginManager'
import { useAutomationStore, WATCH_INTERVAL_MS } from './automationStore'
import { currentMacroContext } from './macroBridge'
import { ScriptDeclarations } from './ScriptDeclarations'
import styles from './MacroConsole.module.css'

/** Explicit local-script runner with a stoppable JavaScript console. */
export function MacroConsole() {
  const { t } = useTranslation()
  const state = useAutomationStore()
  const [page, setPage] = useState<'scripts' | 'plugins' | 'api'>('scripts')
  // Null means "show the stored preference"; a string is the user's unsaved draft.
  const [draft, setDraft] = useState<string | null>(null)
  const open = state.open
  const checkFolders = state.checkFolders
  useEffect(() => {
    if (!open) return
    // Reload on change: the console re-reads its folders while it is open, so a
    // script saved from an editor appears without pressing Reload.
    const timer = setInterval(() => {
      void checkFolders()
    }, WATCH_INTERVAL_MS)
    return () => clearInterval(timer)
  }, [open, checkFolders])
  if (!state.open) return null
  const context = state.running ? state.context : currentMacroContext()
  const busy = state.running || state.reading
  const label = t('automation.historyLabel', {
    name: state.selectedScript || t('automation.custom'),
  })
  const folder = draft ?? state.configuredDirectory
  const blocking = state.problems.some(
    (problem) => problem.severity === 'error',
  )

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
                disabled={busy || !state.source.trim() || blocking}
                onClick={() => void state.run(label)}
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
        {(['scripts', 'plugins', 'api'] as const).map((item) => (
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
      ) : page === 'api' ? (
        <ScriptDeclarations />
      ) : (
        <div className={styles.console}>
          <p className={styles.hint}>{t('automation.trust')}</p>
          <div className={styles.folder}>
            <div>
              <strong>{t('automation.folders')}</strong>
              {state.folders.length === 0 ? (
                <code className={styles.path}>
                  {t('automation.folderPending')}
                </code>
              ) : (
                <ul className={styles.folderList}>
                  {state.folders.map((entry) => (
                    <li key={entry.path}>
                      <span>{t(`automation.folderKind.${entry.kind}`)}</span>{' '}
                      <code className={styles.path}>
                        {entry.path}
                        {entry.exists
                          ? ''
                          : ` · ${t('automation.folderMissing')}`}
                      </code>
                    </li>
                  ))}
                </ul>
              )}
            </div>
            <Button
              disabled={state.loading || busy}
              onClick={() => void state.loadScripts(true)}
            >
              {t(state.loading ? 'automation.reloading' : 'automation.reload')}
            </Button>
          </div>
          <div className={styles.controls}>
            <label className={styles.field}>
              <span>{t('automation.configuredFolder')}</span>
              <input
                value={folder}
                disabled={busy}
                onChange={(event) => setDraft(event.target.value)}
              />
            </label>
            <Button
              disabled={busy || folder === state.configuredDirectory}
              onClick={() => {
                void state
                  .setConfiguredDirectory(folder)
                  .then((saved) => saved && setDraft(null))
              }}
            >
              {t('automation.saveFolder')}
            </Button>
          </div>
          {state.changedOnDisk ? (
            <p role="status" className={styles.hint}>
              {t('automation.reloadedOnChange')}
            </p>
          ) : null}
          <div className={styles.fields}>
            <label className={styles.field}>
              <span>{t('automation.script')}</span>
              <select
                value={
                  state.selectedScript
                    ? `${state.selectedFolder}/${state.selectedScript}`
                    : ''
                }
                disabled={busy || state.loading}
                onChange={(event) => {
                  const [kind, ...rest] = event.target.value.split('/')
                  if (rest.length > 0)
                    void state.openScript(
                      rest.join('/'),
                      kind as typeof state.selectedFolder,
                    )
                }}
              >
                <option value="">{t('automation.custom')}</option>
                {state.scripts.map((script) => (
                  <option
                    key={`${script.folder}/${script.name}`}
                    value={`${script.folder}/${script.name}`}
                  >
                    {script.name} ·{' '}
                    {t(`automation.folderKind.${script.folder}`)}
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
          {state.problems.length ? (
            <ul
              className={styles.warnings}
              aria-label={t('automation.check.title')}
            >
              {state.problems.map((problem, index) => (
                <li
                  key={index}
                  className={
                    problem.severity === 'error' ? styles.error : undefined
                  }
                >
                  {t('automation.check.line', { line: problem.line })}{' '}
                  {t(problem.key, problem.values)}
                </li>
              ))}
            </ul>
          ) : null}
          <p className={styles.hint}>
            {t(
              state.language === 'python'
                ? 'automation.pythonHelp'
                : 'automation.javascriptHelp',
            )}
          </p>
          <p className={styles.hint}>{t('automation.transactionHelp')}</p>
          <div className={styles.outputHeader}>
            <strong>{t('automation.output')}</strong>
            <span role="status">{t(`automation.status.${state.status}`)}</span>
          </div>
          {state.historyLabel ? (
            <p className={styles.hint}>
              {t('automation.historyEntry', { label: state.historyLabel })}
            </p>
          ) : null}
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
