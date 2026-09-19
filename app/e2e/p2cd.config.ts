import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { defineConfig } from '@playwright/test'
import { REPO_ROOT } from './support'

const apiPort = process.env.FC_API_PORT ?? '8813'
const uiPort = process.env.FC_UI_PORT ?? '1446'
const apiURL = `http://127.0.0.1:${apiPort}`
const baseURL = `http://127.0.0.1:${uiPort}`
const configDir = mkdtempSync(join(tmpdir(), 'fc-p2cd-e2e-config-'))
// The kernel starts its autosave worker once at process startup. Changing this
// preference through the API later does not stop that already-running worker.
writeFileSync(
  join(configDir, 'prefs.json'),
  JSON.stringify({
    autosave_seconds: 0,
    recent_limit: 20,
    language: 'en',
    theme: 'light',
  }),
)

/** Browser acceptance uses the freshly built kernel and an isolated profile. */
export default defineConfig({
  testDir: '.',
  testMatch: 'p2cd-*.spec.ts',
  workers: 1,
  timeout: 90_000,
  globalTimeout: 15 * 60 * 1000,
  globalTeardown: './p2cd.teardown.ts',
  metadata: { configDir },
  outputDir: resolve(REPO_ROOT, 'target/p2cd-e2e-results'),
  reporter: 'list',
  use: {
    baseURL,
    viewport: { width: 1440, height: 1000 },
    headless: true,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  webServer: [
    {
      command: `target/debug/fc-devserver --port ${apiPort}`,
      cwd: REPO_ROOT,
      env: { FC_CONFIG_DIR: configDir },
      url: `${apiURL}/health`,
      reuseExistingServer: false,
      timeout: 30_000,
    },
    {
      command: `pnpm exec vite --config e2e/p2cd.vite.config.ts --port ${uiPort} --strictPort`,
      cwd: resolve(REPO_ROOT, 'app'),
      env: {
        FC_DEV_PROXY: apiURL,
        VITE_FC_API: baseURL,
        CHOKIDAR_USEPOLLING: '1',
      },
      url: baseURL,
      reuseExistingServer: false,
      timeout: 30_000,
    },
  ],
})
