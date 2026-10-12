import { create } from 'zustand'

import { TOOLS } from '../editor/tools/registry'
import { i18n } from '../i18n'
import { activeTool } from '../stores/editorStore'
import { useKeymapStore } from '../stores/keymapStore'
import {
  selectActiveTab,
  useUiStore,
  type DialogState,
} from '../stores/uiStore'
import { useWorkspaceStore } from '../stores/workspaceStore'

import { commandHelp } from './commandHelp'
import { getCommand, registerCommand } from './commands'
import { shortcutFor } from './keymap'
import { formatShortcut } from './shortcutSyntax'

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
  'history',
  'spacing',
  'variations',
  'crossDocument',
  'batchRename',
  'workspace',
  'automation',
] as const

/** Stable identifier for one local help article. */
export type HelpTopic = (typeof HELP_TOPICS)[number]

/** Context captured before the help dialog takes keyboard focus. */
export interface HelpContext {
  readonly topic: HelpTopic
  readonly surfaceTitle?: string
  readonly controlTitle?: string
  readonly controlHint?: string
  /** Name of the tool the Edit tab is using, when one is active. */
  readonly toolTitle?: string
  /** That tool's gesture hint, when it has one. */
  readonly toolHint?: string
  /** Title of the command the captured control runs, when it runs one. */
  readonly commandTitle?: string
  /** That command's one-line description. */
  readonly commandSummary?: string
  /** That command's effective binding, already formatted for this platform. */
  readonly commandBinding?: string
  /** Why that command is unavailable right now, when it is. */
  readonly commandReason?: string
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

/**
 * How long the pointer must rest on a control before its quick help appears.
 *
 * Long enough that moving across a toolbar does not flash a bubble per button,
 * short enough to feel like an answer rather than a wait. Keyboard focus uses
 * the same delay, so tabbing through a form does not stutter either.
 */
export const QUICK_HELP_DELAY_MS = 600

/**
 * Grace period between the pointer leaving the control and the bubble closing.
 *
 * The bubble holds two real controls, so the pointer has to be able to travel
 * from the control it describes onto it. A `pointerout` carries no reliable
 * `relatedTarget` for that hop in every engine, so the dismissal waits instead:
 * long enough to cross the 8 px gap, short enough not to linger.
 */
export const QUICK_HELP_LEAVE_MS = 200

/** What the quick-help bubble shows, and where it points. */
export interface QuickHelpTarget {
  readonly topic: HelpTopic
  /** Accessible name of the control the bubble describes. */
  readonly title?: string
  /** The one-line description: a command summary, or the control's own hint. */
  readonly summary: string
  /** The control's effective binding, formatted, when it has one. */
  readonly binding?: string
  /** Why the control's command is unavailable, when it is. */
  readonly reason?: string
  /** Viewport rectangle of the control, for placement. */
  readonly anchor: { top: number; left: number; width: number; height: number }
}

interface QuickHelpState {
  target: QuickHelpTarget | null
  show: (target: QuickHelpTarget) => void
  hide: () => void
}

/** The hover/focus quick-help bubble. Never modal, never focus-stealing. */
export const useQuickHelpStore = create<QuickHelpState>((set) => ({
  target: null,
  show: (target) => set({ target }),
  hide: () => set({ target: null }),
}))

let hoveredControl: HTMLElement | null = null

/** Elements that can carry quick help, in priority order of their annotation. */
const HELP_SELECTOR =
  '[data-help-command], [data-help-topic], [aria-describedby], [aria-description], [title]'

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

/** The description a control carries itself, independent of any command. */
function controlDescription(element: Element): string | undefined {
  return (
    referencedText(element, 'aria-describedby') ||
    element.getAttribute('aria-description')?.trim() ||
    (element instanceof HTMLElement ? element.title.trim() : '') ||
    undefined
  )
}

/** The command a control runs, taken from its `data-help-command` marker. */
function commandContext(element: Element | null): {
  readonly topic?: HelpTopic
  readonly commandTitle?: string
  readonly commandSummary?: string
  readonly commandBinding?: string
  readonly commandReason?: string
} {
  const id = element
    ?.closest<HTMLElement>('[data-help-command]')
    ?.dataset.helpCommand?.trim()
  if (!id) return {}
  const command = getCommand(id)
  const help = commandHelp(id)
  if (!command && !help) return {}
  const binding = command
    ? shortcutFor(command.id, command.shortcut)
    : undefined
  const enabled = command?.when?.() ?? true
  return {
    ...(help
      ? { topic: help.topic, commandSummary: i18n.t(help.summary) }
      : {}),
    ...(command ? { commandTitle: i18n.t(command.title) } : {}),
    ...(binding ? { commandBinding: formatShortcut(binding) } : {}),
    ...(!enabled && command?.disabledReason
      ? { commandReason: i18n.t(command.disabledReason) }
      : {}),
  }
}

/**
 * The tool the Edit tab is using, named for the help surfaces.
 *
 * A tool is only in effect where its keys are: on a Glyph tab that is not in
 * shaped-text mode, which is exactly `useToolShortcuts`' own condition. In text
 * mode the letters go into the run and no tool is active, so naming one would
 * be wrong.
 */
function toolContext(): { toolTitle?: string; toolHint?: string } {
  const tab = selectActiveTab(useUiStore.getState())
  if (tab?.kind !== 'glyph' || tab.textState?.mode === 'text') return {}
  const id = activeTool()
  const tool = TOOLS.find((candidate) => candidate.id === id)
  if (!tool) return {}
  return {
    toolTitle: i18n.t(tool.labelKey),
    ...(tool.hintKey ? { toolHint: i18n.t(tool.hintKey) } : {}),
  }
}

/** The article a modal dialog belongs to. Exhaustive over `DialogState`. */
function dialogTopic(kind: DialogState['kind']): HelpTopic {
  switch (kind) {
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
    case 'compareFonts':
    case 'pasteSpecial':
    case 'relinkComponents':
      return 'crossDocument'
    case 'batchRename':
      return 'batchRename'
  }
}

/** The article the modal currently owning the screen belongs to. */
export function modalTopic(): HelpTopic {
  if (useKeymapStore.getState().open) return 'shortcuts'
  if (useWorkspaceStore.getState().dialogOpen) return 'workspace'
  const ui = useUiStore.getState()
  if (ui.pathPrompt || ui.closePrompt) return 'files'
  if (ui.paletteOpen) return 'shortcuts'
  return ui.dialog ? dialogTopic(ui.dialog.kind) : 'dialog'
}

/** The article the workspace itself is about, with no dialog in the way. */
function workspaceTopic(): HelpTopic {
  return selectActiveTab(useUiStore.getState())?.kind ?? 'welcome'
}

/** Builds the quick-help target for one element, or `null` when it has none. */
export function quickHelpTarget(element: HTMLElement): QuickHelpTarget | null {
  if (!element.isConnected) return null
  const command = commandContext(element)
  const explicit =
    element.closest<HTMLElement>('[data-help-topic]')?.dataset.helpTopic
  const topic: HelpTopic =
    command.topic ??
    (isHelpTopic(explicit)
      ? explicit
      : useUiStore.getState().modalCount > 0
        ? modalTopic()
        : workspaceTopic())
  const title = command.commandTitle ?? fieldTitle(element)
  const description = controlDescription(element)
  // A control whose description is its own name says nothing twice: an icon
  // button labelled and titled "Toggle sidebar", or a recent-file row whose
  // description is the path it already prints. Keep the name and let the topic
  // summary say what the surface is for, which is the only new sentence here.
  const summary =
    command.commandSummary ??
    (description && description !== title
      ? description
      : i18n.t(`contextHelp.topics.${topic}.summary`))
  const bounds = element.getBoundingClientRect()
  return {
    topic,
    summary,
    ...(title ? { title } : {}),
    ...(command.commandBinding ? { binding: command.commandBinding } : {}),
    ...(command.commandReason ? { reason: command.commandReason } : {}),
    anchor: {
      top: bounds.top,
      left: bounds.left,
      width: bounds.width,
      height: bounds.height,
    },
  }
}

/**
 * Tracks live pointer and focus help targets while the shell's help host is
 * mounted, and opens the quick-help bubble after {@link QUICK_HELP_DELAY_MS}.
 *
 * The bubble is suppressed while the quick-help target sits behind an open
 * modal, and while the help article itself is on screen: help about help is
 * noise, and a bubble over a modal would describe something unreachable.
 */
export function installContextHelp(): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined
  let leaving: ReturnType<typeof setTimeout> | undefined
  /** The control the open bubble describes, for the Tab hand-off below. */
  let owner: HTMLElement | null = null
  const cancel = () => {
    if (timer !== undefined) clearTimeout(timer)
    timer = undefined
  }
  const cancelLeave = () => {
    if (leaving !== undefined) clearTimeout(leaving)
    leaving = undefined
  }
  const dismiss = () => {
    cancel()
    cancelLeave()
    owner = null
    if (useQuickHelpStore.getState().target) useQuickHelpStore.getState().hide()
  }
  const clear = () => {
    hoveredControl = null
    dismiss()
  }
  /**
   * Whether an event happened inside the bubble itself.
   *
   * The bubble holds two real controls, so every dismissal has to leave them
   * alone: a `pointerdown` that unmounted the bubble would cancel the `click`
   * that was about to activate "Open help article", and a key press would end
   * the interaction the reader had just started.
   */
  const insideBubble = (target: EventTarget | null): boolean =>
    target instanceof Element && target.closest('[data-quick-help]') !== null
  const reachable = (element: HTMLElement): boolean => {
    if (useContextHelpStore.getState().context) return false
    if (element.closest('[data-quick-help]')) return false
    const modal = element.closest('[role="dialog"]')
    return useUiStore.getState().modalCount === 0 || modal !== null
  }
  const arm = (element: HTMLElement | null) => {
    cancel()
    if (!element || !reachable(element)) return
    timer = setTimeout(() => {
      timer = undefined
      if (!reachable(element)) return
      const target = quickHelpTarget(element)
      if (!target) return
      owner = element
      useQuickHelpStore.getState().show(target)
    }, QUICK_HELP_DELAY_MS)
  }
  const over = (event: PointerEvent) => {
    // The pointer reaching the bubble is the reader using it, not leaving the
    // control: keep what is on screen and stop the pending dismissal.
    if (insideBubble(event.target)) {
      cancelLeave()
      return
    }
    const next =
      event.target instanceof Element
        ? event.target.closest<HTMLElement>(HELP_SELECTOR)
        : null
    cancelLeave()
    // Arriving anywhere other than the control the bubble belongs to closes it.
    if (next !== owner) dismiss()
    hoveredControl = next
    arm(next)
  }
  const out = (event: PointerEvent) => {
    if (insideBubble(event.relatedTarget)) return
    if (
      event.relatedTarget instanceof Node &&
      hoveredControl?.contains(event.relatedTarget)
    )
      return
    // The F1 hover target goes at once, as it always has: leaving a control
    // means F1 is about the workspace again. Only the bubble waits, to give the
    // pointer time to land on it; `over` cancels that the moment it arrives.
    hoveredControl = null
    cancel()
    cancelLeave()
    leaving = setTimeout(dismiss, QUICK_HELP_LEAVE_MS)
  }
  const focused = (event: FocusEvent) => {
    // Focus moving into the bubble is the reader using it, not leaving it.
    if (insideBubble(event.target)) return
    hoveredControl = null
    dismiss()
    arm(
      event.target instanceof Element
        ? event.target.closest<HTMLElement>(HELP_SELECTOR)
        : null,
    )
  }
  const onPointerDown = (event: PointerEvent) => {
    if (insideBubble(event.target)) return
    dismiss()
  }
  const onKeyDown = (event: KeyboardEvent) => {
    const open = useQuickHelpStore.getState().target !== null
    // Escape dismisses the bubble without disturbing the dialog behind it.
    if (event.key === 'Escape') {
      if (open) event.stopPropagation()
      dismiss()
      return
    }
    // Once focus is in the bubble, its own keys are the reader's: Tab moves
    // between its two controls, Enter and Space activate them.
    if (open && insideBubble(event.target)) return
    // Tab from the control the bubble describes hands focus to the bubble, the
    // way an interactive tooltip has to sit next to its trigger in the focus
    // order. It is rendered in a portal, so the order has to be made rather
    // than inherited. A second Tab leaves it and the focus change dismisses it.
    if (
      open &&
      event.key === 'Tab' &&
      !event.shiftKey &&
      event.target === owner
    ) {
      const entry = document
        .querySelector('[data-quick-help]')
        ?.querySelector<HTMLElement>('button')
      if (entry) {
        event.preventDefault()
        entry.focus()
        return
      }
    }
    // Anything else the reader types means they have moved on.
    dismiss()
  }
  document.addEventListener('pointerover', over)
  document.addEventListener('pointerout', out)
  document.addEventListener('pointerdown', onPointerDown)
  document.addEventListener('focusin', focused)
  document.addEventListener('keydown', onKeyDown, true)
  window.addEventListener('blur', clear)
  return () => {
    clear()
    document.removeEventListener('pointerover', over)
    document.removeEventListener('pointerout', out)
    document.removeEventListener('pointerdown', onPointerDown)
    document.removeEventListener('focusin', focused)
    document.removeEventListener('keydown', onKeyDown, true)
    window.removeEventListener('blur', clear)
  }
}

/** Opens help for the focused control, visible modal or active workspace tab. */
export function openContextHelp(): void {
  if (useContextHelpStore.getState().context) return
  useQuickHelpStore.getState().hide()
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
  const command = commandContext(control)
  const explicit =
    control?.closest<HTMLElement>('[data-help-topic]')?.dataset.helpTopic
  const topic: HelpTopic =
    command.topic ??
    (isHelpTopic(explicit)
      ? explicit
      : dialog
        ? modalTopic()
        : workspaceTopic())
  // A control that runs a command is described by its command entry; its own
  // `aria-description` is that same sentence, so repeating it as a "focused
  // control" block would print the summary twice.
  const controlHint =
    control && command.commandSummary === undefined
      ? controlDescription(control)
      : undefined
  useContextHelpStore.getState().open({
    topic,
    ...(dialog ? { surfaceTitle: accessibleTitle(dialog) } : {}),
    ...(controlHint && control
      ? { controlTitle: fieldTitle(control), controlHint }
      : {}),
    ...command,
    ...(dialog ? {} : toolContext()),
  })
}

/** Opens the help article at one topic, from the quick-help bubble. */
export function openHelpArticle(topic: HelpTopic): void {
  useQuickHelpStore.getState().hide()
  useContextHelpStore.getState().open({ topic })
}

/**
 * Localized screen-reader description and help marker for a control.
 *
 * The visible tooltip is the quick-help bubble, so this deliberately does not
 * set `title`: a native tooltip would duplicate it a few hundred milliseconds
 * later and hide the bubble's "Open help article" action behind itself.
 */
export function helpHint(topic: HelpTopic): {
  'aria-description': string
  'data-help-topic': HelpTopic
} {
  return {
    'aria-description': i18n.t(`contextHelp.topics.${topic}.summary`),
    'data-help-topic': topic,
  }
}

/** Marks a control as running one command, so help can describe that command. */
export function helpCommand(id: string): { 'data-help-command': string } {
  return { 'data-help-command': id }
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
