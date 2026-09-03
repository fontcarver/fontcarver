import type { ButtonHTMLAttributes, ReactNode } from 'react'

import styles from './controls.module.css'

export interface IconButtonProps extends Omit<
  ButtonHTMLAttributes<HTMLButtonElement>,
  'aria-label'
> {
  /** Accessible name; icon buttons never carry visible text. */
  label: string
  children: ReactNode
}

/** A compact square button holding a glyph or icon, always labelled. */
export function IconButton({
  label,
  className,
  type = 'button',
  children,
  ...rest
}: IconButtonProps) {
  return (
    <button
      {...rest}
      type={type}
      aria-label={label}
      title={label}
      className={
        className ? `${styles.iconButton} ${className}` : styles.iconButton
      }
    >
      {children}
    </button>
  )
}
