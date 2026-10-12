import { execFileSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = fileURLToPath(new URL('../..', import.meta.url))

/** Runs a prebuilt CLI without compiling or changing caches during a test. */
export function runCli(args: readonly string[]): string {
  const configured = process.env.FC_E2E_CLI
  const executable =
    process.platform === 'win32' ? 'fontcarver.exe' : 'fontcarver'
  const candidates = configured
    ? [resolve(repoRoot, configured)]
    : [
        resolve(repoRoot, 'target/debug', executable),
        resolve(repoRoot, 'target/release', executable),
      ]
  const binary = candidates.find((candidate) => existsSync(candidate))
  if (!binary)
    throw new Error(
      'E2E needs a prebuilt fontcarver CLI. Run cargo build -p fc-cli before testing, ' +
        'or set FC_E2E_CLI to its executable path. No build is started by the suite.',
    )
  return execFileSync(binary, args, {
    cwd: repoRoot,
    encoding: 'utf8',
    maxBuffer: 32 * 1024 * 1024,
  })
}
