import { useEffect, useId, useRef, type ReactNode } from 'react'

import styles from '../shell/dialog.module.css'

/** Dialog widths from the design spec: 480, 560 and 640 pixels. */
export type DialogSize = 'sm' | 'md' | 'lg'

const SIZE_CLASS: Record<DialogSize, string> = {
  sm: styles.dialogSm,
  md: styles.dialogMd,
  lg: styles.dialogLg,
}

const FOCUSABLE =
  'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])'

export interface DialogProps {
  /** Visible title, also the dialog's accessible name. */
  title: string
  size?: DialogSize
  /** Buttons of the footer row, right-aligned by default. */
  footer?: ReactNode
  /** Whether the footer spreads its children to both edges. */
  footerSpread?: boolean
  /** Whether the body scrolls once it outgrows the viewport. */
  scrollBody?: boolean
  /** Escape, the overlay and every Cancel button end up here. */
  onCancel: () => void
  /** Enter confirms unless the focused control owns the key itself. */
  onConfirm?: () => void
  children: ReactNode
}

/**
 * The modal primitive every FontWorkshop dialog is built from: 6 px radius,
 * 24 px padding, a 16/600 title, Escape cancels, Enter confirms and Tab is
 * trapped inside. Focus moves to the first control on open and returns to the
 * element that had it when the dialog closes.
 */
export function Dialog({
  title,
  size = 'md',
  footer,
  footerSpread = false,
  scrollBody = false,
  onCancel,
  onConfirm,
  children,
}: DialogProps) {
  const titleId = useId()
  const dialogRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const dialog = dialogRef.current
    const restore =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null
    // Autofocus wins when a dialog marks its own entry point; otherwise the
    // first control takes focus so the trap always has something inside it.
    if (dialog && !dialog.contains(document.activeElement))
      dialog.querySelector<HTMLElement>(FOCUSABLE)?.focus()
    return () => restore?.focus()
  }, [])

  const trapTab = (event: React.KeyboardEvent): void => {
    const items = [
      ...(dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE) ?? []),
    ]
    if (items.length === 0) return
    const first = items[0]
    const last = items[items.length - 1]
    const active = document.activeElement
    if (
      event.shiftKey &&
      (active === first || !dialogRef.current?.contains(active))
    ) {
      event.preventDefault()
      last?.focus()
    } else if (!event.shiftKey && active === last) {
      event.preventDefault()
      first?.focus()
    }
  }

  const onKeyDown = (event: React.KeyboardEvent): void => {
    if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      onCancel()
      return
    }
    if (event.key === 'Tab') {
      trapTab(event)
      return
    }
    if (event.key !== 'Enter' || !onConfirm) return
    // A textarea, a button and a link all own Enter themselves; confirming from
    // them would either swallow a newline or fire two actions.
    const target = event.target
    if (!(target instanceof HTMLElement)) return
    const tag = target.tagName
    if (tag === 'TEXTAREA' || tag === 'BUTTON' || tag === 'A') return
    event.preventDefault()
    onConfirm()
  }

  const bodyClass = scrollBody
    ? `${styles.dialogBody} ${styles.dialogScroll}`
    : styles.dialogBody
  const footerClass = footerSpread
    ? `${styles.dialogFooter} ${styles.dialogFooterSpread}`
    : styles.dialogFooter

  return (
    <div
      className={styles.overlay}
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onCancel()
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className={`${styles.dialog} ${SIZE_CLASS[size]}`}
        onKeyDown={onKeyDown}
      >
        <div className={styles.dialogHeader}>
          <h2 id={titleId} className={styles.dialogTitle}>
            {title}
          </h2>
        </div>
        <div className={bodyClass}>{children}</div>
        {footer ? <div className={footerClass}>{footer}</div> : null}
      </div>
    </div>
  )
}
