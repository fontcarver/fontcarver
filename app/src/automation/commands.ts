import { registerCommands } from '../shell/commands'
import { openMacroConsole } from './automationStore'

/** Installs the shell entry for the script console and bundled plugins. */
export function registerAutomationCommands(): void {
  registerCommands([
    {
      id: 'automation.console',
      title: 'automation.title',
      menu: 'view',
      group: 'automation',
      run: openMacroConsole,
    },
  ])
}
