import { existsSync } from 'node:fs'
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
 * has built neither must first run `cargo build -p fc-devserver`.
 */
const devserver =
  [
    `${repoRoot}target/release/fc-devserver`,
    `${repoRoot}target/debug/fc-devserver`,
  ].find((binary) => existsSync(binary)) ??
  `${repoRoot}target/debug/fc-devserver`

export default defineConfig({
  testDir: './e2e',
  metadata: { kernelBinary: devserver, kernelPort: apiPort },
  // Each test owns a kernel process on the proxy port; keep these lifetimes
  // sequential. Browser contexts and source documents are isolated as well.
  workers: 1,
  fullyParallel: false,
  // A red CI run has to be diagnosable from the artifact alone: the default
  // reporter writes nothing to disk, so CI gets the HTML report the workflow
  // uploads, and one retry so `on-first-retry` can actually record a trace.
  reporter: process.env.CI ? [['html', { open: 'never' }], ['list']] : 'list',
  retries: process.env.CI ? 1 : 0,
  forbidOnly: !!process.env.CI,
  // Bound a hung suite below the workflow's own step timeout, with room for
  // cold imports in each test; the optional CJK spec budgets four minutes.
  globalTimeout: 25 * 60 * 1000,
  use: {
    baseURL,
    // The Edit tab stacks the text bar, the spacing and kerning chrome, the
    // canvas floor and the preview strip: below about 950 px of height it
    // scrolls, and a spec that computes canvas coordinates then drags off
    // screen. The three P2 configurations already run at this size.
    viewport: { width: 1440, height: 1000 },
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  webServer: [
    {
      command: `pnpm exec vite --port ${uiPort} --strictPort`,
      url: baseURL,
      env: { FC_DEV_PROXY: apiURL, VITE_FC_API: baseURL },
      reuseExistingServer: false,
    },
  ],
})
