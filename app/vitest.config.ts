import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
    setupFiles: ['./src/test/setup.ts'],
    /*
     * Keep individual test runs small enough to coexist with other tasks.
     *
     * Each worker is a fork with its own jsdom and its own Vite transform
     * pipeline. A percentage-of-cores cap still starts ten workers per run on
     * a 20-core machine, multiplying contention during parallel development.
     * Use two workers by default; a dedicated CI run can override this with
     * --maxWorkers. Focused parallel tasks should use --maxWorkers=1.
     */
    maxWorkers: 2,
    minWorkers: 1,
  },
})
