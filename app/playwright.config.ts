import { defineConfig } from '@playwright/test'

/**
 * The UI port is configurable so a second checkout (or a parallel agent) can
 * run the suite without fighting over the default one. `FW_DEV_PROXY` points
 * Vite at the kernel development server; see `vite.config.ts`.
 */
const port = process.env.FW_UI_PORT ?? '1420'
const baseURL = `http://localhost:${port}`

export default defineConfig({
  testDir: './e2e',
  use: {
    baseURL,
  },
  webServer: {
    command: `pnpm exec vite --port ${port} --strictPort`,
    url: baseURL,
    reuseExistingServer: !process.env.CI,
  },
})
