import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { initialUiState, useUiStore } from '../stores/uiStore'

import { Dialog } from './Dialog'

const SRC = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const TOKENS = readFileSync(resolve(SRC, 'styles/tokens.css'), 'utf8')
const DIALOG_CSS = readFileSync(resolve(SRC, 'shell/dialog.module.css'), 'utf8')

/** Resolves `var(--token)` references against the light-theme token block. */
function resolveTokens(value: string): string {
  return value.replace(/var\((--[\w-]+)\)/g, (_match, name: string) => {
    const declared = new RegExp(`${name}:\\s*([^;]+);`).exec(TOKENS)
    return (declared?.[1] ?? '').trim()
  })
}

/**
 * Every rule of the dialog stylesheet, by the selectors it is written for.
 *
 * Rules are looked up by selector rather than by where they sit in the file, so
 * reordering the stylesheet cannot silently point an assertion at a different
 * rule. The stylesheet has no at-rules, which this flat parse relies on.
 */
const RULES = [
  ...DIALOG_CSS.replace(/\/\*[\s\S]*?\*\//g, '').matchAll(
    /([^{}]+)\{([^}]*)\}/g,
  ),
].map(([, selectors = '', body = '']) => ({
  selectors: selectors
    .split(',')
    .map((one) => one.trim().replace(/\s+/g, ' '))
    .filter((one) => one.length > 0),
  body,
}))

/** One declaration of one rule of the dialog stylesheet, with tokens resolved. */
function declaration(selector: string, property: string): string {
  const rule = RULES.find((candidate) => candidate.selectors.includes(selector))
  expect(rule, `rule ${selector} exists`).toBeTruthy()
  const found = new RegExp(`(?:^|;)\\s*${property}:\\s*([^;]+)`).exec(
    rule?.body ?? '',
  )
  expect(found, `${selector} declares ${property}`).toBeTruthy()
  return resolveTokens((found?.[1] ?? '').trim())
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
})

describe('Dialog chrome', () => {
  // The design spec fixes these numbers for every dialog; the primitive is the
  // only place they are written down, and the P0 prompts keep their own chrome.
  it.each(['.dialogSm', '.dialogMd', '.dialogLg'])(
    '%s uses 24 px padding, a 16/600 title and a 14 px body',
    (size) => {
      expect(declaration(`${size} .dialogHeader`, 'padding')).toBe(
        '24px 24px 0',
      )
      expect(declaration(`${size} .dialogBody`, 'padding')).toBe('12px 24px')
      expect(declaration(`${size} .dialogBody`, 'font-size')).toBe('14px')
      expect(declaration(`${size} .dialogFooter`, 'padding')).toBe(
        '12px 24px 24px',
      )
      expect(declaration(`${size} .dialogTitle`, 'font-size')).toBe('16px')
      expect(declaration(`${size} .dialogTitle`, 'font-weight')).toBe('600')
    },
  )

  it('parses the stylesheet flatly, which needs it to stay at-rule free', () => {
    expect(DIALOG_CSS).not.toMatch(/@[a-z-]+/)
  })

  it('leaves the P0 prompts on their own chrome', () => {
    expect(declaration('.dialogHeader', 'padding')).toBe('12px 16px 0')
    expect(declaration('.dialogTitle', 'font-size')).toBe('15px')
  })
})

describe('Dialog behaviour', () => {
  function setup(children = <input aria-label="Field" />) {
    const onCancel = vi.fn()
    const onConfirm = vi.fn()
    render(
      <Dialog
        title="Test dialog"
        onCancel={onCancel}
        onConfirm={onConfirm}
        footer={<button type="button">Confirm</button>}
      >
        {children}
      </Dialog>,
    )
    return { onCancel, onConfirm }
  }

  it('cancels on Escape and confirms on Enter outside a text area', async () => {
    const user = userEvent.setup()
    const { onCancel, onConfirm } = setup()

    await user.click(screen.getByLabelText('Field'))
    await user.keyboard('{Enter}')
    expect(onConfirm).toHaveBeenCalledTimes(1)

    await user.keyboard('{Escape}')
    expect(onCancel).toHaveBeenCalledTimes(1)
  })

  it('leaves Enter to the control that owns it', async () => {
    const user = userEvent.setup()
    const { onConfirm } = setup(<textarea aria-label="Recipe" />)

    await user.click(screen.getByLabelText('Recipe'))
    await user.keyboard('{Enter}')

    expect(onConfirm).not.toHaveBeenCalled()
  })

  it('registers as a modal while it is mounted', () => {
    expect(useUiStore.getState().modalCount).toBe(0)
    const { unmount } = render(
      <Dialog title="Modal" onCancel={() => undefined}>
        <input aria-label="Field" />
      </Dialog>,
    )

    expect(useUiStore.getState().modalCount).toBe(1)

    unmount()
    expect(useUiStore.getState().modalCount).toBe(0)
  })
})
