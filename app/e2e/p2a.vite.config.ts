import { defineConfig, mergeConfig } from 'vite'

import appConfig from '../vite.config'

/** Keep an acceptance page stable while independent agents edit the checkout. */
export default defineConfig(mergeConfig(appConfig, { server: { hmr: false } }))
