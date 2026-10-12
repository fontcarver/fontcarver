import { spawn } from 'node:child_process'
import {
  copyFileSync,
  createWriteStream,
  mkdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { basename, join } from 'node:path'
import { expect, test as base } from '@playwright/test'
import type { DocSummary } from '../src/api/generated/DocSummary'
import { fixture, REPO_ROOT, requestApi } from './support'

type SessionFixtures = {
  fixtureFont: string | null
  isolatedSession: DocSummary | null
}

/** Each test owns a kernel process, documents, preferences and font sidecars. */
export const test = base.extend<SessionFixtures>({
  fixtureFont: [fixture('DejaVuSans.ttf'), { option: true }],
  isolatedSession: [
    async ({ request, context, fixtureFont }, runTest, testInfo) => {
      const { kernelBinary, kernelPort } = testInfo.config.metadata
      if (typeof kernelBinary !== 'string' || typeof kernelPort !== 'string')
        throw new Error(
          'the Playwright configuration must name its kernel binary and port',
        )
      const directory = testInfo.outputPath('fixture-font')
      const configDir = testInfo.outputPath('kernel-config')
      mkdirSync(configDir, { recursive: true })
      // The autosave worker reads this once at startup, before any test edits.
      writeFileSync(
        join(configDir, 'prefs.json'),
        JSON.stringify({
          autosave_seconds: 0,
          recent_limit: 20,
          language: 'en',
          theme: 'light',
        }),
      )
      const log = createWriteStream(testInfo.outputPath('kernel.log'))
      const kernel = spawn(kernelBinary, ['--port', kernelPort], {
        cwd: REPO_ROOT,
        env: { ...process.env, FC_CONFIG_DIR: configDir },
        stdio: ['ignore', 'pipe', 'pipe'],
      })
      let ready = false
      let startupError: Error | undefined
      kernel.on('error', (error) => {
        startupError = error
      })
      const stopped = new Promise<void>((resolve) =>
        kernel.once('close', () => resolve()),
      )
      for (const output of [kernel.stdout, kernel.stderr]) {
        output.on('data', (chunk: Buffer) => {
          log.write(chunk)
          ready ||= chunk
            .toString()
            .includes('FontCarver development server listening')
        })
      }
      try {
        // Check our child's own startup message before probing health. An
        // occupied port must fail, never attach to somebody else's server.
        await expect
          .poll(
            async () => {
              if (startupError) throw startupError
              if (kernel.exitCode !== null || kernel.signalCode !== null)
                throw new Error('kernel exited before startup; see kernel.log')
              if (!ready) return false
              return (
                await request.get(`http://127.0.0.1:${kernelPort}/health`)
              ).ok()
            },
            { timeout: 25_000, intervals: [100, 250] },
          )
          .toBe(true)
        let doc: DocSummary | null = null
        if (fixtureFont) {
          mkdirSync(directory, { recursive: true })
          const path = join(directory, basename(fixtureFont))
          // Workspace sidecars belong to the test, not the committed font.
          copyFileSync(fixtureFont, path)
          doc = await requestApi<DocSummary>(request, 'session.open', {
            path,
            face_index: null,
          })
        }
        await runTest(doc)
      } finally {
        try {
          await context.close()
        } finally {
          // Closing a page cannot cancel a font parser already running in the
          // kernel. Terminating this owned process prevents late registration
          // and other unfinished requests from reaching the next test.
          if (kernel.exitCode === null && kernel.signalCode === null)
            kernel.kill()
          const force = setTimeout(() => kernel.kill('SIGKILL'), 5_000)
          try {
            await stopped
          } finally {
            clearTimeout(force)
          }
          await new Promise<void>((resolve) => log.end(resolve))
          rmSync(directory, { recursive: true, force: true })
          rmSync(configDir, { recursive: true, force: true })
        }
      }
    },
    // Import/startup and disposal do not consume the browser action budget.
    { auto: true, timeout: 30_000 },
  ],
})

/** Source workflows open their own fonts and need no DejaVu document. */
export const sourceTest = test.extend({ fixtureFont: null })
