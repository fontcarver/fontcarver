import type { ButtonHTMLAttributes, ReactNode } from 'react'

import styles from './controls.module.css'

/** Visual weight of a button. */
export type ButtonVariant = 'default' | 'primary' | 'ghost'

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant
  large?: boolean
  children: ReactNode
}

/** A standard text button. */
export function Button({
  variant = 'default',
  large = false,
  className,
  type = 'button',
  children,
  ...rest
}: ButtonProps) {
  const classes = [styles.button]
  if (variant === 'primary') classes.push(styles.primary)
  if (variant === 'ghost') classes.push(styles.ghost)
  if (large) classes.push(styles.large)
  if (className) classes.push(className)

  return (
    <button {...rest} type={type} className={classes.join(' ')}>
      {children}
    </button>
  )
}
