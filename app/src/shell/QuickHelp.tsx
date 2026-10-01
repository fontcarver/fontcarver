import { useTranslation } from 'react-i18next'
import { createPortal } from 'react-dom'

import { useKeymapStore } from '../stores/keymapStore'

import {
  openHelpArticle,
  useQuickHelpStore,
  type QuickHelpTarget,
} from './contextHelp'
import { getCommand } from './commands'
import { shortcutFor } from './keymap'
import { formatShortcut } from './shortcutSyntax'
import styles from './QuickHelp.module.css'

/** Width of the bubble, matching `--quick-help-width` in the stylesheet. */
const WIDTH = 280
/** Gap between the bubble and the control it describes. */
const GAP = 8

/** Places the bubble under its control, inside the viewport. */
function position(anchor: QuickHelpTarget['anchor']): {
  top: number
  left: number
} {
  const viewport = {
    width: globalThis.innerWidth || WIDTH + 2 * GAP,
    height: globalThis.innerHeight || 0,
  }
  const left = Math.max(
    GAP,
    Math.min(anchor.left, Math.max(GAP, viewport.width - WIDTH - GAP)),
  )
  const below = anchor.top + anchor.height + GAP
  // Flip above the control when there is not enough room underneath; the
  // bubble's own height is unknown before layout, so reserve a sensible band.
  const top =
    viewport.height > 0 && below + 140 > viewport.height
      ? Math.max(GAP, anchor.top - 140 - GAP)
      : below
  return { top, left }
}

/**
 * Quick Help: the non-modal half of row 107.
 *
 * It appears after a hover or focus delay on any control that carries a help
 * annotation, names the control and its command, gives the one-line
 * description and the live binding, and offers the full article. It never takes
 * focus — a tooltip that stole focus would break the gesture that opened it —
 * and it is removed by any pointer press, key press or focus change.
 */
export function QuickHelp() {
  const { t } = useTranslation()
  const target = useQuickHelpStore((state) => state.target)
  const hide = useQuickHelpStore((state) => state.hide)
  const overrides = useKeymapStore((state) => state.overrides)

  if (!target) return null
  const help = getCommand('help.context')
  const helpBinding = shortcutFor('help.context', help?.shortcut, overrides)
  const { top, left } = position(target.anchor)

  return createPortal(
    <div
      data-quick-help
      role="status"
      aria-live="polite"
      aria-label={t('contextHelp.quickHelp')}
      className={styles.bubble}
      style={{ top, left }}
    >
      {target.title ? <p className={styles.title}>{target.title}</p> : null}
      <p className={styles.summary}>{target.summary}</p>
      {target.binding ? (
        <p className={styles.binding}>
          {t('contextHelp.binding', { shortcut: target.binding })}
        </p>
      ) : null}
      {target.reason ? (
        <p className={styles.reason}>
          {t('contextHelp.unavailableHere', { reason: target.reason })}
        </p>
      ) : null}
      <div className={styles.actions}>
        <button
          type="button"
          className={styles.more}
          onClick={() => openHelpArticle(target.topic)}
        >
          {t('contextHelp.openArticle')}
        </button>
        <span className={styles.hint}>
          {helpBinding
            ? t('contextHelp.quickHelpHint', {
                shortcut: formatShortcut(helpBinding),
              })
            : t('contextHelp.quickHelpHintNoKey')}
        </span>
      </div>
      <button
        type="button"
        className={styles.dismiss}
        aria-label={t('contextHelp.close')}
        onClick={hide}
      >
        ×
      </button>
    </div>,
    document.body,
  )
}
