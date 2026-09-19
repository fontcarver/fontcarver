import { defineConfig, mergeConfig } from 'vite'
import appConfig from '../vite.config'

/** Parallel implementation changes must not reload an acceptance scenario. */
export default defineConfig(
  mergeConfig(appConfig, {
    server: { hmr: false, watch: { usePolling: true, interval: 1000 } },
  }),
)
