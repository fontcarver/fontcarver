import { useTranslation } from 'react-i18next'

import { selectProgress, useProgressStore } from '../stores/progressStore'
import { selectActiveDoc, useUiStore } from '../stores/uiStore'

import styles from './AppShell.module.css'

const STAGE_KEYS: Readonly<Record<string, string>> = {
  compile: 'progress.compile',
  preserve: 'progress.preserve',
  write: 'progress.write',
}

/**
 * The 2 px accent line under the tab bar that reports a long-running kernel
 * operation. Nothing renders while the kernel is idle, and the line is never a
 * modal spinner: the rest of the shell stays usable while it runs.
 */
export function ProgressLine() {
  const { t } = useTranslation()
  const activeDoc = useUiStore(selectActiveDoc)
  const progress = useProgressStore(selectProgress(activeDoc))

  if (!progress) return null

  const stageKey = STAGE_KEYS[progress.stage]
  const stage = stageKey ? t(stageKey) : progress.stage
  return (
    <div
      role="progressbar"
      aria-label={t('a11y.progress', { stage })}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={progress.percent}
      data-stage={progress.stage}
      className={styles.progressLine}
    >
      <div
        className={styles.progressFill}
        style={{ width: `${progress.percent}%` }}
      />
    </div>
  )
}
