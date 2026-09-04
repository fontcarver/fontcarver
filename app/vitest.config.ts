import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
    setupFiles: ['./src/test/setup.ts'],
    /*
     * Half the cores, not all of them.
     *
     * Each worker is a fork with its own jsdom and its own Vite transform
     * pipeline, so one worker per core oversubscribes the machine: measured on
     * a 20-core box, the suite takes ~44 s at the default and ~16-26 s capped
     * at 50 %, and only the uncapped run pushes individual component tests past
     * their timeouts (a `GlyphView` test that takes 233 ms on its own was
     * measured at 1 579 ms under the default pool). The cap is the fix for
     * those timeouts; raising the timeouts only hid the contention.
     */
    maxWorkers: '50%',
  },
})
