import { beforeEach, describe, expect, it } from 'vitest'

import en from '../i18n/en.json'
import { initialUiState, useUiStore } from '../stores/uiStore'

import { registerBuiltinCommands } from './builtinCommands'
import { commandHelp, commandHelpIds } from './commandHelp'
import { listCommands, resetCommands } from './commands'
import { HELP_TOPICS, type HelpTopic } from './contextHelp'
import { registerWorkspaceTabCommands } from './workspaceTabCommands'

/** Every command the shell registers at start-up, in registration order. */
function everyCommand() {
  resetCommands()
  registerBuiltinCommands()
  registerWorkspaceTabCommands()
  return listCommands()
}

/** Whether a dotted key resolves to a string in the English catalogue. */
function hasEnglish(key: string): boolean {
  let node: unknown = en
  for (const part of key.split('.')) {
    if (typeof node !== 'object' || node === null) return false
    node = (node as Record<string, unknown>)[part]
  }
  return typeof node === 'string' && node.length > 0
}

beforeEach(() => {
  useUiStore.setState(initialUiState())
})

describe('command help coverage', () => {
  it('gives every registered command a help entry', () => {
    const missing = everyCommand()
      .filter((command) => commandHelp(command.id) === undefined)
      .map((command) => command.id)
    expect(missing).toEqual([])
  })

  it('names a real help topic in every entry', () => {
    const topics = new Set<string>(HELP_TOPICS)
    const wrong = everyCommand()
      .map((command) => [command.id, commandHelp(command.id)] as const)
      .filter(([, help]) => help && !topics.has(help.topic))
      .map(([id, help]) => `${id} -> ${help?.topic ?? '?'}`)
    expect(wrong).toEqual([])
  })

  it('resolves every entry summary in the English catalogue', () => {
    const missing = everyCommand()
      .map((command) => commandHelp(command.id)?.summary)
      .filter((key): key is string => typeof key === 'string')
      .filter((key) => !hasEnglish(key))
    expect([...new Set(missing)]).toEqual([])
  })

  it('has no entry for a command that no longer exists', () => {
    const commands = everyCommand()
    const reachable = new Set(
      commandHelpIds().filter((id) =>
        commands.some(
          (command) => command.id === id || command.id.startsWith(`${id}.`),
        ),
      ),
    )
    const stale = commandHelpIds().filter((id) => !reachable.has(id))
    expect(stale).toEqual([])
  })

  it('translates the title of every registered command', () => {
    const missing = everyCommand()
      .filter((command) => !hasEnglish(command.title))
      .map((command) => `${command.id} -> ${command.title}`)
    expect(missing).toEqual([])
  })

  it('explains every gated command', () => {
    const silent = everyCommand()
      .filter((command) => command.when !== undefined)
      .filter(
        (command) =>
          command.disabledReason === undefined ||
          !hasEnglish(command.disabledReason),
      )
      .map((command) => command.id)
    expect(silent).toEqual([])
  })
})

describe('help topics', () => {
  it('ships four strings for every topic', () => {
    const incomplete: string[] = []
    for (const topic of HELP_TOPICS)
      for (const part of ['title', 'summary', 'first', 'second', 'third'])
        if (!hasEnglish(`contextHelp.topics.${topic}.${part}`))
          incomplete.push(`${topic}.${part}`)
    expect(incomplete).toEqual([])
  })

  it('ships no article for a topic the code cannot reach', () => {
    const shipped = Object.keys(
      (en as { contextHelp: { topics: Record<string, unknown> } }).contextHelp
        .topics,
    )
    const known = new Set<string>(HELP_TOPICS)
    expect(shipped.filter((topic) => !known.has(topic as HelpTopic))).toEqual(
      [],
    )
  })
})
