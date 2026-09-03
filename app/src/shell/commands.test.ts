import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  commandsForMenu,
  getCommand,
  isCommandEnabled,
  listCommands,
  registerCommand,
  resetCommands,
  runCommand,
  setCommandErrorReporter,
  unregisterCommand,
} from './commands'

afterEach(() => {
  resetCommands()
  setCommandErrorReporter(() => undefined)
})

describe('command registry', () => {
  it('registers, lists and runs a command', () => {
    const run = vi.fn()
    registerCommand({ id: 'test.run', title: 'cmd.test', menu: 'file', run })

    expect(listCommands().map((command) => command.id)).toEqual(['test.run'])
    expect(getCommand('test.run')?.title).toBe('cmd.test')
    expect(commandsForMenu('file')).toHaveLength(1)
    expect(commandsForMenu('view')).toHaveLength(0)

    expect(runCommand('test.run')).toBe(true)
    expect(run).toHaveBeenCalledOnce()
  })

  it('refuses to run unknown or disabled commands', () => {
    const run = vi.fn()
    registerCommand({
      id: 'test.gated',
      title: 'cmd.test',
      when: () => false,
      run,
    })

    expect(runCommand('test.missing')).toBe(false)
    expect(isCommandEnabled('test.missing')).toBe(false)
    expect(runCommand('test.gated')).toBe(false)
    expect(isCommandEnabled('test.gated')).toBe(false)
    expect(run).not.toHaveBeenCalled()
  })

  it('keeps registration order and replaces a re-registered id', () => {
    registerCommand({ id: 'a', title: 'a', run: () => undefined })
    registerCommand({ id: 'b', title: 'b', run: () => undefined })
    registerCommand({ id: 'a', title: 'a2', run: () => undefined })

    expect(listCommands().map((command) => command.id)).toEqual(['a', 'b'])
    expect(getCommand('a')?.title).toBe('a2')

    unregisterCommand('a')
    expect(getCommand('a')).toBeUndefined()
  })

  it('routes synchronous and asynchronous failures to the error reporter', async () => {
    const reporter = vi.fn()
    setCommandErrorReporter(reporter)
    registerCommand({
      id: 'test.throws',
      title: 'x',
      run: () => {
        throw new Error('sync boom')
      },
    })
    registerCommand({
      id: 'test.rejects',
      title: 'x',
      run: () => Promise.reject(new Error('async boom')),
    })

    runCommand('test.throws')
    runCommand('test.rejects')
    await Promise.resolve()

    expect(reporter).toHaveBeenCalledTimes(2)
  })
})
