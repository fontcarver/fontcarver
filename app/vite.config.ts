import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

/**
 * Optional same-origin proxy for the kernel development server.
 *
 * `fw-devserver` only allows the default UI origin, so a UI served on another
 * port (a second agent, a second checkout, a parallel e2e run) is blocked by
 * CORS. Setting `FW_DEV_PROXY=http://localhost:<port>` routes `/api` and
 * `/events` through Vite instead, which makes the traffic same-origin; point
 * `VITE_FW_API` at the Vite origin in that case.
 */
const proxyTarget = process.env.FW_DEV_PROXY

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: 1420,
    strictPort: true,
    proxy: proxyTarget
      ? {
          '/api': { target: proxyTarget, changeOrigin: true },
          '/events': { target: proxyTarget, ws: true },
        }
      : undefined,
  },
})
