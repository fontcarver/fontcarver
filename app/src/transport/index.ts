import { HttpTransport } from './HttpTransport'
import { TauriTransport } from './TauriTransport'
import type { Transport } from './Transport'

export { HttpTransport } from './HttpTransport'
export { TauriTransport } from './TauriTransport'
export { ApiCallError, type Transport } from './Transport'

export function createTransport(): Transport {
  if ('__TAURI_INTERNALS__' in window) return new TauriTransport()
  return new HttpTransport(
    import.meta.env.VITE_FC_API ?? 'http://localhost:8787',
  )
}
