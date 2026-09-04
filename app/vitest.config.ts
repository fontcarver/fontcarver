import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
    setupFiles: ['./src/test/setup.ts'],
    // Component tests drive real user-event sequences through jsdom; under a
    // fully parallel run on a loaded machine the slowest of them sit close to
    // the 5 s default, which turns a passing suite into a coin flip.
    testTimeout: 20_000,
  },
})
