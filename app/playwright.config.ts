import { existsSync, rmSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

import { defineConfig } from '@playwright/test'

/**
 * The end-to-end suite is self-contained: it starts the kernel development
 * server and Vite itself, so `pnpm e2e` needs nothing but
 * `cargo build -p fc-devserver` and `pnpm install`.
 *
 * Both ports are configurable so a second checkout (or a parallel agent) can
 * run the suite without fighting over the defaults. The UI is served the kernel
 * through Vite's proxy (`FC_DEV_PROXY`) and told to call its own origin
 * (`VITE_FC_API`), which keeps the traffic same-origin whatever port is in use
 * — `fc-devserver` only allows the default UI origin across origins.
 */
const apiPort = process.env.FC_API_PORT ?? '8787'
const uiPort = process.env.FC_UI_PORT ?? '1420'
// Both servers bind the IPv4 loopback, and `localhost` resolves to `::1`
// first on some runners, so every URL here names the address explicitly.
const baseURL = `http://127.0.0.1:${uiPort}`
const apiURL = `http://127.0.0.1:${apiPort}`

const repoRoot = fileURLToPath(new URL('..', import.meta.url))

/**
 * The pre-built binary when there is one, so a run costs no compilation, and
 * the release one first: only a release kernel opens the 65,535-glyph corpus
 * the CJK spec wants inside the transport's request timeout. A checkout that
 * has built neither falls back to Cargo, which is why the kernel server gets a
 * longer start-up budget than Vite.
 */
const devserver =
  [
    `${repoRoot}target/release/fc-devserver`,
    `${repoRoot}target/debug/fc-devserver`,
  ].find((binary) => existsSync(binary)) ??
  'cargo run --quiet -p fc-devserver --'

/**
 * The kernel's configuration home for the run, emptied as the suite starts.
 *
 * Preferences and crash-recovery snapshots live under it, so the suite is given
 * one of its own inside `target/`: a snapshot an editing spec leaves behind
 * cannot reach the developer's own Font Cutter, and "a clean launch" means the
 * same thing on a machine that has run the suite before as on one that has not.
 */
const configDir = `${repoRoot}target/e2e-config-${apiPort}`
rmSync(configDir, { recursive: true, force: true })

/** The font every spec but the CJK one is written against. */
const fixture = 'fixtures/fonts/DejaVuSans.ttf'

export default defineConfig({
  testDir: './e2e',
  // A red CI run has to be diagnosable from the artifact alone: the default
  // reporter writes nothing to disk, so CI gets the HTML report the workflow
  // uploads, and one retry so `on-first-retry` can actually record a trace.
  reporter: process.env.CI ? [['html', { open: 'never' }], ['list']] : 'list',
  retries: process.env.CI ? 1 : 0,
  forbidOnly: !!process.env.CI,
  // Bounds a hung suite well below the workflow's own step timeout; the
  // slowest spec (CJK, when it runs at all) budgets four minutes of that.
  globalTimeout: 15 * 60 * 1000,
  use: {
    baseURL,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  webServer: [
    {
      command: `${devserver} --port ${apiPort} --open ${fixture}`,
      cwd: repoRoot,
      env: { FC_CONFIG_DIR: configDir },
      url: `${apiURL}/health`,
      // `/health` answers before any document is open, so a `fc-devserver`
      // already running on this port without the fixture satisfies the probe
      // and every spec then fails on a missing document. Stop a stray server
      // before running the suite, or point `FC_API_PORT` somewhere else.
      reuseExistingServer: !process.env.CI,
      timeout: 180_000,
      stdout: 'pipe',
      stderr: 'pipe',
    },
    {
      command: `pnpm exec vite --port ${uiPort} --strictPort`,
      url: baseURL,
      env: { FC_DEV_PROXY: apiURL, VITE_FC_API: baseURL },
      reuseExistingServer: !process.env.CI,
    },
  ],
})
