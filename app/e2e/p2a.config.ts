import { resolve } from 'node:path'

import { defineConfig } from '@playwright/test'

import { REPO_ROOT } from './support'

const apiPort = process.env.FC_API_PORT ?? '8799'
const uiPort = process.env.FC_UI_PORT ?? '1432'
const apiURL = `http://127.0.0.1:${apiPort}`
const baseURL = `http://127.0.0.1:${uiPort}`
/** Small source-font acceptance uses the freshly built debug kernel. */
export default defineConfig({
  testDir: '.',
  metadata: {
    kernelBinary: resolve(REPO_ROOT, 'target/debug/fc-devserver'),
    kernelPort: apiPort,
  },
  testMatch: 'p2a-kerning.spec.ts',
  workers: 1,
  timeout: 90_000,
  globalTimeout: 10 * 60 * 1000,
  outputDir: resolve(REPO_ROOT, 'target/p2a-e2e-results'),
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
      command: `pnpm exec vite --config e2e/p2a.vite.config.ts --port ${uiPort} --strictPort`,
      cwd: resolve(REPO_ROOT, 'app'),
      env: { FC_DEV_PROXY: apiURL, VITE_FC_API: baseURL },
      url: baseURL,
      reuseExistingServer: false,
      timeout: 30_000,
    },
  ],
})
