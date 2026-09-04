import { existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

import { defineConfig } from '@playwright/test'

/**
 * The end-to-end suite is self-contained: it starts the kernel development
 * server and Vite itself, so `pnpm e2e` needs nothing but
 * `cargo build -p fw-devserver` and `pnpm install`.
 *
 * Both ports are configurable so a second checkout (or a parallel agent) can
 * run the suite without fighting over the defaults. The UI is served the kernel
 * through Vite's proxy (`FW_DEV_PROXY`) and told to call its own origin
 * (`VITE_FW_API`), which keeps the traffic same-origin whatever port is in use
 * — `fw-devserver` only allows the default UI origin across origins.
 */
const apiPort = process.env.FW_API_PORT ?? '8787'
const uiPort = process.env.FW_UI_PORT ?? '1420'
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
    `${repoRoot}target/release/fw-devserver`,
    `${repoRoot}target/debug/fw-devserver`,
  ].find((binary) => existsSync(binary)) ??
  'cargo run --quiet -p fw-devserver --'

/** The font every spec but the CJK one is written against. */
const fixture = 'fixtures/fonts/DejaVuSans.ttf'

export default defineConfig({
  testDir: './e2e',
  use: {
    baseURL,
  },
  webServer: [
    {
      command: `${devserver} --port ${apiPort} --open ${fixture}`,
      cwd: repoRoot,
      url: `${apiURL}/health`,
      reuseExistingServer: !process.env.CI,
      timeout: 180_000,
      stdout: 'pipe',
      stderr: 'pipe',
    },
    {
      command: `pnpm exec vite --port ${uiPort} --strictPort`,
      url: baseURL,
      env: { FW_DEV_PROXY: apiURL, VITE_FW_API: baseURL },
      reuseExistingServer: !process.env.CI,
    },
  ],
})
