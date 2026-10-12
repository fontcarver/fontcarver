import { defineConfig, mergeConfig } from 'vite'
import appConfig from '../vite.config'

/** Parallel implementation edits must not navigate an acceptance page. */
export default defineConfig(
  mergeConfig(appConfig, {
    server: { hmr: false, watch: { usePolling: true, interval: 1000 } },
  }),
)
