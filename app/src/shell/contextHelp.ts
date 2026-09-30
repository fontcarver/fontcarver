import { create } from 'zustand'

import { i18n } from '../i18n'
import { useKeymapStore } from '../stores/keymapStore'
import { selectActiveTab, useUiStore } from '../stores/uiStore'

import { registerCommand } from './commands'

/** Local help topics, available without a network connection. */
export const HELP_TOPICS = [
  'welcome',
  'font',
  'glyph',
  'bitmap',
  'fontinfo',
  'features',
  'export',
  'preferences',
  'shortcuts',
  'newFont',
  'addGlyphs',
  'problems',
  'recovery',
  'outline',
  'files',
  'dialog',
] as const

/** Stable identifier for one local help article. */
export type HelpTopic = (typeof HELP_TOPICS)[number]

/** Context captured before the help dialog takes keyboard focus. */
export interface HelpContext {
  readonly topic: HelpTopic
  readonly surfaceTitle?: string
  readonly controlTitle?: string
  readonly controlHint?: string
}

interface ContextHelpState {
  context: HelpContext | null
  open: (context: HelpContext) => void
  close: () => void
}

/** Help has its own transient state so it never replaces an editing dialog. */
export const useContextHelpStore = create<ContextHelpState>((set, get) => ({
  context: null,
  open: (context) => {
    if (get().context === null) set({ context })
  },
  close: () => set({ context: null }),
}))

let hoveredControl: HTMLElement | null = null

/** Tracks live pointer help targets while the shell's help host is mounted. */
export function installContextHelp(): () => void {
  const clear = () => {
    hoveredControl = null
  }
  const over = (event: PointerEvent) => {
    hoveredControl =
      event.target instanceof Element
        ? event.target.closest<HTMLElement>(
            '[data-help-topic], [aria-describedby], [aria-description], [title]',
          )
        : null
  }
  const out = (event: PointerEvent) => {
    if (
      !(event.relatedTarget instanceof Node) ||
      !hoveredControl?.contains(event.relatedTarget)
    )
      clear()
  }
  document.addEventListener('pointerover', over)
  document.addEventListener('pointerout', out)
  document.addEventListener('focusin', clear)
  window.addEventListener('blur', clear)
  return () => {
    clear()
    document.removeEventListener('pointerover', over)
    document.removeEventListener('pointerout', out)
    document.removeEventListener('focusin', clear)
    window.removeEventListener('blur', clear)
  }
}

function isHelpTopic(value: string | undefined): value is HelpTopic {
  return HELP_TOPICS.some((topic) => topic === value)
}

function referencedText(
  element: Element,
  attribute: string,
): string | undefined {
  const ids = element.getAttribute(attribute)?.trim().split(/\s+/) ?? []
  const text = ids
    .map((id) => element.ownerDocument.getElementById(id)?.textContent?.trim())
    .filter(Boolean)
    .join(' ')
  return text || undefined
}

function accessibleTitle(element: Element): string | undefined {
  return (
    referencedText(element, 'aria-labelledby') ||
    element.getAttribute('aria-label')?.trim() ||
    undefined
  )
}

function fieldTitle(element: HTMLElement): string | undefined {
  const accessible = accessibleTitle(element)
  if (accessible) return accessible
  if (
    element instanceof HTMLInputElement ||
    element instanceof HTMLTextAreaElement ||
    element instanceof HTMLSelectElement
  ) {
    return (
      Array.from(element.labels ?? [])
        .map((label) => label.textContent?.trim())
        .filter(Boolean)
        .join(' ') || undefined
    )
  }
  if (element instanceof HTMLButtonElement) {
    return element.textContent?.trim() || undefined
  }
  return undefined
}

function modalTopic(): HelpTopic {
  if (useKeymapStore.getState().open) return 'shortcuts'
  const ui = useUiStore.getState()
  if (ui.pathPrompt || ui.closePrompt) return 'files'
  if (ui.paletteOpen) return 'shortcuts'
  switch (ui.dialog?.kind) {
    case 'export':
      return 'export'
    case 'preferences':
      return 'preferences'
    case 'newFont':
      return 'newFont'
    case 'addGlyphs':
      return 'addGlyphs'
    case 'findProblems':
      return 'problems'
    case 'recovery':
      return 'recovery'
    case 'simplify':
    case 'expandStroke':
    case 'styleEffects':
      return 'outline'
    default:
      return 'dialog'
  }
}

/** Opens help for the focused control, visible modal or active workspace tab. */
export function openContextHelp(): void {
  if (useContextHelpStore.getState().context) return
  const active =
    document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null
  const dialogs = [
    ...document.querySelectorAll<HTMLElement>('[role="dialog"]'),
  ].filter(
    (dialog) => !dialog.closest('[hidden], [aria-hidden="true"], [inert]'),
  )
  const dialog = active?.closest('[role="dialog"]') ?? dialogs.at(-1)
  const editing = active?.matches(
    'input, textarea, select, [contenteditable="true"]',
  )
  const hovered =
    !editing &&
    hoveredControl?.isConnected &&
    (!dialog || dialog.contains(hoveredControl))
      ? hoveredControl
      : null
  const control = hovered ?? active
  const explicit =
    control?.closest<HTMLElement>('[data-help-topic]')?.dataset.helpTopic
  const topic: HelpTopic = isHelpTopic(explicit)
    ? explicit
    : dialog
      ? modalTopic()
      : (selectActiveTab(useUiStore.getState())?.kind ?? 'welcome')
  const controlHint = control
    ? referencedText(control, 'aria-describedby') ||
      control.getAttribute('aria-description')?.trim() ||
      control.title.trim() ||
      undefined
    : undefined
  useContextHelpStore.getState().open({
    topic,
    ...(dialog ? { surfaceTitle: accessibleTitle(dialog) } : {}),
    ...(controlHint && control
      ? { controlTitle: fieldTitle(control), controlHint }
      : {}),
  })
}

/** Localized pointer and screen-reader hints for a help-enabled control. */
export function helpHint(topic: HelpTopic): {
  title: string
  'aria-description': string
  'data-help-topic': HelpTopic
} {
  const description = i18n.t(`contextHelp.topics.${topic}.summary`)
  return {
    title: description,
    'aria-description': description,
    'data-help-topic': topic,
  }
}

/** Adds context help to the Help menu, command palette and modal-safe shortcuts. */
export function registerHelpCommands(): void {
  registerCommand({
    id: 'help.context',
    title: 'contextHelp.command',
    menu: 'help',
    group: 'reference',
    shortcut: 'F1',
    allowInInput: true,
    allowInModal: true,
    run: openContextHelp,
  })
}
