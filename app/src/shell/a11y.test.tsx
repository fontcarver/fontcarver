import { readFileSync, readdirSync } from 'node:fs'
import { dirname, join, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { CloseDocumentDialog } from './CloseDocumentDialog'
import { PathPromptDialog } from './PathPromptDialog'
import { ShortcutSettingsDialog } from './ShortcutSettingsDialog'
import { ContextHelpDialog } from './ContextHelpDialog'
import { registerCommands, resetCommands } from './commands'
import { openContextHelp, useContextHelpStore } from './contextHelp'
import { useKeymapStore } from '../stores/keymapStore'
import { initialUiState, useUiStore, type DialogState } from '../stores/uiStore'
import { setTransport, type DocId } from '../api'
import { DialogHost } from '../dialogs/DialogHost'
import { DEFAULT_PREFS } from '../stores/prefsStore'
import { useSessionStore } from '../stores/sessionStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'

const SRC = resolve(dirname(fileURLToPath(import.meta.url)), '..')

function componentFiles(directory: string = SRC): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name)
    if (entry.isDirectory()) return componentFiles(path)
    return entry.name.endsWith('.tsx') && !entry.name.includes('.test.')
      ? [path]
      : []
  })
}

/** The text of a JSX opening tag that starts at `start` in `source`. */
function openingTag(
  source: string,
  start: number,
): { tag: string; end: number } {
  let index = start
  let depth = 0
  while (index < source.length) {
    const character = source[index]
    if (character === '{') depth += 1
    else if (character === '}') depth -= 1
    else if (character === '>' && depth === 0) break
    index += 1
  }
  return { tag: source.slice(start, index + 1), end: index }
}

interface Finding {
  readonly file: string
  readonly line: number
  readonly detail: string
}

/** Escapes a captured identifier for use inside a `RegExp`. */
function escapeForRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function findings(
  check: (source: string, report: (at: number, detail: string) => void) => void,
) {
  const found: Finding[] = []
  for (const path of componentFiles()) {
    const source = readFileSync(path, 'utf8')
    check(source, (at, detail) =>
      found.push({
        file: relative(SRC, path),
        line: source.slice(0, at).split('\n').length,
        detail,
      }),
    )
  }
  return found.map((entry) => `${entry.file}:${entry.line} ${entry.detail}`)
}

describe('accessibility of the component source', () => {
  it('gives every icon-only button an accessible name', () => {
    expect(
      findings((source, report) => {
        // `<button>` and the two primitives built on it. `IconButton` takes a
        // required `label`, so it is named by construction; `Button` is not, and
        // an icon-only `<Button>` is exactly as unreachable as a bare one.
        for (const match of source.matchAll(/<(button|Button)\b/g)) {
          const name = match[1] ?? 'button'
          const { tag, end } = openingTag(source, match.index)
          if (/aria-label|aria-labelledby/.test(tag)) continue
          if (name === 'Button' && /\blabel=/.test(tag)) continue
          if (/\/>\s*$/.test(tag)) {
            report(
              match.index,
              `self-closing <${name}> with no accessible name`,
            )
            continue
          }
          const close = source.indexOf(`</${name}>`, end)
          const children = close > 0 ? source.slice(end + 1, close) : ''
          // Anything marked `aria-hidden` is decoration, so a button whose only
          // remaining content is such a node has no accessible name.
          const visible = children
            .replace(/<[^>]*aria-hidden[^>]*>[\s\S]*?<\/[a-zA-Z]+>/g, '')
            .replace(/<[^>]*aria-hidden[^>]*\/>/g, '')
          if (!/[A-Za-z0-9一-鿿]/.test(visible))
            report(
              match.index,
              `icon-only <${name}> without an accessible name`,
            )
        }
      }),
    ).toEqual([])
  })

  it('labels every form field', () => {
    expect(
      findings((source, report) => {
        for (const match of source.matchAll(/<(input|select|textarea)\b/g)) {
          const { tag } = openingTag(source, match.index)
          if (/aria-label|aria-labelledby|type="hidden"/.test(tag)) continue
          // A field that is not focusable *and* hidden from the accessibility
          // tree is driven by a labelled control of its own — the file input
          // behind an Import button. Either alone is not enough.
          if (/tabIndex={-1}/.test(tag) && /aria-hidden/.test(tag)) continue
          // An `id` only names a field when something points `htmlFor` at it.
          const id = /\bid={?[`'"]?([\w$.{}-]+)/.exec(tag)?.[1]
          if (
            id &&
            new RegExp(`htmlFor={?[\`'"]?${escapeForRegExp(id)}`).test(source)
          )
            continue
          if (id && /htmlFor={`[^`]*\${/.test(source)) continue
          const before = source.slice(0, match.index)
          const label = Math.max(
            before.lastIndexOf('<label'),
            before.lastIndexOf('<Field'),
          )
          const closed = Math.max(
            before.lastIndexOf('</label>'),
            before.lastIndexOf('</Field>'),
          )
          if (label <= closed)
            report(match.index, `<${match[1]}> with no label, id or aria-label`)
        }
      }),
    ).toEqual([])
  })

  it('keeps every click target reachable from the keyboard', () => {
    expect(
      findings((source, report) => {
        const elements =
          /<(div|span|li|td|tr|p|section|article|img|label|a|h1|h2|h3|h4)\b/g
        for (const match of source.matchAll(elements)) {
          const { tag } = openingTag(source, match.index)
          if (!/onClick/.test(tag)) continue
          // `tabIndex={-1}` takes an element *out* of the tab order, so it is
          // not a keyboard path; only a non-negative value is.
          const tabbable = /tabIndex=/.test(tag) && !/tabIndex={\s*-/.test(tag)
          if (
            tabbable ||
            /onKeyDown|onKeyUp|role="button"|role="tab"|role="option"|role="menuitem"/.test(
              tag,
            )
          )
            continue
          report(match.index, `<${match[1]}> has onClick but no keyboard path`)
        }
      }),
    ).toEqual([])
  })

  it('gives every hand-built dialog a name, a modal flag and key handling', () => {
    // A dialog that builds its own markup instead of using the `Dialog`
    // primitive still owes its reader an accessible name, `aria-modal` and the
    // Escape/Tab handling that `useModalChrome` supplies.
    expect(
      findings((source, report) => {
        for (const match of source.matchAll(/<div\b/g)) {
          const { tag } = openingTag(source, match.index)
          if (!/role="dialog"/.test(tag)) continue
          const missing = [
            /aria-modal/.test(tag) ? '' : 'aria-modal',
            /aria-label|aria-labelledby/.test(tag) ? '' : 'an accessible name',
            /onKeyDown|\.\.\.chrome/.test(tag) ? '' : 'key handling',
          ].filter(Boolean)
          if (missing.length > 0)
            report(match.index, `role="dialog" without ${missing.join(', ')}`)
        }
      }),
    ).toEqual([])
  })
})

const transport = {
  calls: [] as { method: string }[],
}

beforeEach(() => {
  localStorage.clear()
  transport.calls = []
  useUiStore.setState(initialUiState())
  useKeymapStore.setState({ overrides: {}, allowConflicts: [], open: false })
  useContextHelpStore.getState().close()
  resetCommands()
  registerCommands([
    {
      id: 'file.open',
      title: 'cmd.file.open',
      shortcut: 'Mod+O',
      run: vi.fn(),
    },
  ])
})

afterEach(() => {
  resetCommands()
  useKeymapStore.setState({ overrides: {}, allowConflicts: [], open: false })
})

/** Every focusable control of the rendered dialog, in document order. */
function focusables(dialog: HTMLElement): HTMLElement[] {
  return [
    ...dialog.querySelectorAll<HTMLElement>(
      'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])',
    ),
  ]
}

/** The accessible name of one control, by the rules a screen reader applies. */
function accessibleName(element: HTMLElement): string {
  const labelledBy = element.getAttribute('aria-labelledby')
  if (labelledBy)
    return labelledBy
      .split(/\s+/)
      .map((id) => document.getElementById(id)?.textContent?.trim() ?? '')
      .join(' ')
      .trim()
  const label = element.getAttribute('aria-label')?.trim()
  if (label) return label
  if (
    element instanceof HTMLInputElement ||
    element instanceof HTMLSelectElement ||
    element instanceof HTMLTextAreaElement
  ) {
    const own = [...(element.labels ?? [])]
      .map((node) => node.textContent?.trim() ?? '')
      .join(' ')
      .trim()
    if (own) return own
    return element.getAttribute('placeholder')?.trim() ?? ''
  }
  return element.textContent?.trim() ?? ''
}

describe('the shell dialogs that build their own modal markup', () => {
  it('counts the close prompt as a modal, traps Tab and restores focus', async () => {
    const user = userEvent.setup()
    const summary = makeSummary({ dirty: true })
    const doc = summary.doc
    useSessionStore.setState({ docs: [summary] })
    const outside = document.createElement('button')
    document.body.append(outside)
    outside.focus()
    const view = render(<CloseDocumentDialog />)
    // The host stays mounted and renders nothing until a prompt arrives. While
    // it shows nothing it must not count as a modal: a leaked count stands the
    // whole global shortcut dispatcher down.
    expect(useUiStore.getState().modalCount).toBe(0)
    useUiStore.setState({ closePrompt: { doc } })
    view.rerender(<CloseDocumentDialog />)

    const dialog = screen.getByRole('dialog')
    expect(useUiStore.getState().modalCount).toBe(1)
    const controls = focusables(dialog)
    expect(controls.length).toBeGreaterThan(1)
    expect(controls.map(accessibleName).filter((name) => name === '')).toEqual(
      [],
    )
    expect(dialog.contains(document.activeElement)).toBe(true)

    controls[controls.length - 1]?.focus()
    await user.tab()
    expect(dialog.contains(document.activeElement)).toBe(true)

    await user.keyboard('{Escape}')
    expect(useUiStore.getState().closePrompt).toBeNull()
    // Focus returns to the control the prompt interrupted.
    expect(document.activeElement).toBe(outside)
    view.unmount()
    expect(useUiStore.getState().modalCount).toBe(0)
    outside.remove()
  })

  it('counts the path prompt as a modal and names every control', async () => {
    const user = userEvent.setup()
    const idle = render(<PathPromptDialog />)
    expect(useUiStore.getState().modalCount).toBe(0)
    idle.unmount()
    useUiStore.setState({ pathPrompt: { mode: 'open' } })
    render(<PathPromptDialog />)
    const dialog = screen.getByRole('dialog')
    expect(useUiStore.getState().modalCount).toBe(1)
    expect(
      focusables(dialog)
        .map(accessibleName)
        .filter((name) => name === ''),
    ).toEqual([])
    await user.keyboard('{Escape}')
    expect(useUiStore.getState().pathPrompt).toBeNull()
  })

  it('names every control of the shortcut editor and the help article', () => {
    useKeymapStore.setState({ open: true })
    const editor = render(<ShortcutSettingsDialog />)
    const unnamed = focusables(screen.getByRole('dialog'))
      .map((control) => [control.tagName, accessibleName(control)] as const)
      .filter(([, name]) => name === '')
    expect(unnamed).toEqual([])
    editor.unmount()
    useKeymapStore.setState({ open: false })

    render(<ContextHelpDialog />)
    act(openContextHelp)
    const help = screen.getByRole('dialog')
    expect(
      focusables(help)
        .map(accessibleName)
        .filter((name) => name === ''),
    ).toEqual([])
  })
})

/**
 * Every dialog `DialogHost` can mount, with the state each one needs.
 *
 * The rendered walk below opens all of them rather than a sample, because the
 * host is the one place a new dialog is added and a missing accessible name is
 * exactly what nobody notices there.
 */
const DIALOG_KINDS: readonly DialogState[] = [
  { kind: 'compareFonts', doc: 1 as DocId },
  { kind: 'pasteSpecial', doc: 1 as DocId, glyphs: ['A'] },
  { kind: 'relinkComponents', doc: 1 as DocId, glyphs: ['A'] },
  { kind: 'newFont' },
  { kind: 'batchRename', doc: 1 as DocId, glyphs: ['A'] },
  { kind: 'addGlyphs', doc: 1 as DocId },
  { kind: 'export', doc: 1 as DocId },
  { kind: 'findProblems', doc: 1 as DocId },
  { kind: 'preferences' },
  { kind: 'recovery', entries: [] },
  { kind: 'simplify' },
  { kind: 'expandStroke' },
  { kind: 'styleEffects', doc: 1 as DocId, glyphs: ['A'], master: 'm1' },
] as unknown as readonly DialogState[]

describe('every dialog the host can mount', () => {
  beforeEach(() => {
    setTransport(
      createMockTransport({
        'session.list': () => [makeSummary()],
        'session.recent': () => [],
        'doc.summary': () => makeSummary(),
        'prefs.get': () => DEFAULT_PREFS,
        'prefs.set': () => DEFAULT_PREFS,
      }),
    )
    useSessionStore.setState({ docs: [makeSummary()] })
  })
  afterEach(() => setTransport(undefined))

  it.each(DIALOG_KINDS.map((dialog) => [dialog.kind, dialog] as const))(
    'names every control of the %s dialog',
    (_kind, dialog) => {
      useUiStore.setState({ dialog })
      const view = render(<DialogHost />)
      // Every kind must actually render: a host that quietly produced nothing
      // would make this walk pass by checking no dialog at all.
      const node = screen.getByRole('dialog')
      expect(node).toHaveAttribute('aria-modal', 'true')
      expect(accessibleName(node)).not.toBe('')
      const unnamed = focusables(node)
        .map((control) => [control.tagName, accessibleName(control)] as const)
        .filter(([, name]) => name === '')
      expect(unnamed).toEqual([])
      view.unmount()
      useUiStore.setState({ dialog: null })
    },
  )
})
