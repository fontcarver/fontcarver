import { execFileSync } from 'node:child_process'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import type { Page } from '@playwright/test'

/** Repository root, resolved from this file so the specs need no cwd. */
export const REPO_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '..',
  '..',
)

/** Absolute path of a committed test font. */
export function fixture(name: string): string {
  return resolve(REPO_ROOT, 'fixtures', 'fonts', name)
}

interface ResultEnvelope<T> {
  result?: T
  error?: { code: string; message: string }
}

/**
 * Calls one kernel method from inside the page.
 *
 * The specs drive the same HTTP transport the application uses, from the same
 * origin, so a step a later task will perform with the pointer can be performed
 * here through the API without a second code path.
 */
export async function api<T>(
  page: Page,
  method: string,
  params: unknown,
): Promise<T> {
  const envelope = await page.evaluate(
    async ([method, params]: [string, unknown]) => {
      const response = await fetch(`/api/${method}`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(params),
      })
      return (await response.json()) as unknown
    },
    [method, params] as [string, unknown],
  )
  const result = envelope as ResultEnvelope<T>
  if (result.error) throw new Error(`${method} failed: ${result.error.message}`)
  return result.result as T
}

/** Runs `fw inspect --json` and returns the parsed report. */
export function inspect(path: string): {
  summary: { glyph_count: number; family_name: string; units_per_em: number }
} {
  const stdout = execFileSync(
    'cargo',
    [
      'run',
      '-q',
      '-p',
      'fw-cli',
      '--bin',
      'fw',
      '--',
      'inspect',
      '--json',
      path,
    ],
    { cwd: REPO_ROOT, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 },
  )
  return JSON.parse(stdout) as ReturnType<typeof inspect>
}
